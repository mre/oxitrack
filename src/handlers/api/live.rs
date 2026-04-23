use axum::{
    extract::{Query, State},
    response::Html,
};
use axum_ctx::{RespErrCtx, RespErrExt, RespResult, StatusCode};

use crate::{handlers::stats_data::DateRange, states::AppState};

pub async fn get(
    State(state): AppState,
    Query(range): Query<DateRange>,
) -> RespResult<Html<String>> {
    let start_utc = range
        .start_datetime()
        .map(|pdt| crate::handlers::stats_data::local_to_utc(pdt, state.utc_offset));
    let end_utc = range
        .end_datetime()
        .map(|pdt| crate::handlers::stats_data::local_to_utc(pdt, state.utc_offset));

    // Total visits for the current range (OOB-swapped into #total-visits).
    let total: i64 = sqlx::query_scalar(
        r"SELECT COUNT(*) FROM visits
          WHERE (? IS NULL OR registered_at >= ?)
            AND (? IS NULL OR registered_at < ?)",
    )
    .bind(start_utc)
    .bind(start_utc)
    .bind(end_utc)
    .bind(end_utc)
    .fetch_one(&state.pool)
    .await
    .ctx(StatusCode::INTERNAL_SERVER_ERROR)
    .log_msg("Live total-visits query failed!")?;

    // Active path strings for row dots (OOB-swapped into #live-path-set).
    let path_ids = state.visitor_states.live_path_ids();
    let active_paths: Vec<String> = if path_ids.is_empty() {
        vec![]
    } else {
        let placeholders = path_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("SELECT path FROM paths WHERE id IN ({placeholders})");
        let mut q = sqlx::query_scalar::<_, String>(&sql);
        for id in &path_ids {
            q = q.bind(id);
        }
        q.fetch_all(&state.pool)
            .await
            .ctx(StatusCode::INTERNAL_SERVER_ERROR)
            .log_msg("Live active-paths query failed!")?
    };

    // Serialise path list as a JSON array for the JS handler to read.
    let paths_json = {
        let items = active_paths
            .iter()
            .map(|p| format!("\"{}\"", p.replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(",");
        format!("[{items}]")
    };

    let count = state.visitor_states.live_count();
    let live_html = live_indicator(count);

    let total_oob = format!(r#"<span id="total-visits" hx-swap-oob="true">{total}</span>"#);
    let paths_oob =
        format!(r#"<div id="live-path-set" hx-swap-oob="true" data-paths='{paths_json}'></div>"#);

    Ok(Html(format!("{live_html}{total_oob}{paths_oob}")))
}

fn live_indicator(count: usize) -> String {
    if count == 0 {
        r#"<span class="live-indicator live-indicator--dim" title="No visitors right now"><span class="live-dot"></span></span>"#
            .to_string()
    } else {
        let s = if count == 1 { "" } else { "s" };
        format!(
            r#"<span class="live-indicator" title="{count} visitor{s} on site right now"><span class="live-dot"></span>{count}</span>"#
        )
    }
}
