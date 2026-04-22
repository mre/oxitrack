use axum::{
    extract::{Query, State},
    response::Html,
};
use axum_ctx::{RespErrCtx, RespErrExt, RespResult, StatusCode};
use serde::Deserialize;
use time::{Date, PrimitiveDateTime, Time};

use crate::{handlers::stats_data::local_to_utc, states::AppState};

#[derive(Deserialize)]
pub struct LiveQuery {
    pub from: Option<String>,
    pub to: Option<String>,
}

pub async fn get(State(state): AppState, Query(q): Query<LiveQuery>) -> RespResult<Html<String>> {
    let count = state.visitor_states.live_count();

    // Parse the date range sent by the frontend (same format as the stats panel).
    let fmt = time::macros::format_description!("[year]-[month]-[day]");
    let start_utc = q
        .from
        .filter(|s| !s.is_empty())
        .and_then(|s| Date::parse(&s, fmt).ok())
        .map(|d| local_to_utc(PrimitiveDateTime::new(d, Time::MIDNIGHT), state.utc_offset));
    let end_utc =
        q.to.filter(|s| !s.is_empty())
            .and_then(|s| Date::parse(&s, fmt).ok())
            .map(|d| {
                local_to_utc(
                    PrimitiveDateTime::new(d + time::Duration::days(1), Time::MIDNIGHT),
                    state.utc_offset,
                )
            });

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

    let live_html = if count == 0 {
        r#"<span class="live-indicator live-indicator--dim" title="No visitors right now"><span class="live-dot"></span></span>"#
            .to_string()
    } else {
        let s = if count == 1 { "" } else { "s" };
        format!(
            r#"<span class="live-indicator" title="{count} visitor{s} on site right now"><span class="live-dot"></span>{count}</span>"#
        )
    };

    // OOB swap updates the total-visits counter in the stat line without a full page reload.
    let oob = format!(r#"<span id="total-visits" hx-swap-oob="true">{total}</span>"#);

    Ok(Html(format!("{live_html}{oob}")))
}
