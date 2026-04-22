use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Query, State};
use axum_ctx::*;
use serde::Deserialize;

use crate::{
    handlers::{
        count_rows::CountRows,
        dashboard::page_stats::{self, PageStat},
        stats_data::{DateRange, build_chart, referrer_count::ReferrerCount},
    },
    states::AppState,
};

#[derive(Deserialize, Default)]
pub struct HxStatsQuery {
    pub from: Option<String>,
    pub to: Option<String>,
    pub path: Option<String>,
}

#[derive(Template, WebTemplate)]
#[template(path = "hx_stats.html")]
pub struct HxStats {
    pub base_url: &'static str,
    pub pages: CountRows<PageStat>,
    pub referrers: CountRows<ReferrerCount>,
    pub chart: Vec<crate::handlers::stats_data::ChartBar>,
    pub range: DateRange,
    pub total_visits: i64,
    pub path: Option<String>,
}

pub async fn get(State(state): AppState, Query(q): Query<HxStatsQuery>) -> RespResult<HxStats> {
    let range = DateRange::from_params(q.from, q.to);
    let now = state.now_tz()?;

    let path_id: Option<i64> = if let Some(ref path) = q.path {
        sqlx::query_scalar("SELECT id FROM paths WHERE path = ?")
            .bind(path)
            .fetch_optional(&state.pool)
            .await
            .ctx(StatusCode::INTERNAL_SERVER_ERROR)
            .log_msg("Failed to query path id")?
    } else {
        None
    };

    let (page_stats_vec, mut referrers_vec, chart) = tokio::try_join!(
        page_stats::all_sorted_by_count(state, &range, now),
        ReferrerCount::all_sorted_by_count(
            state,
            path_id,
            range.start_datetime(),
            range.end_datetime()
        ),
        build_chart(state, path_id, &range, now),
    )?;

    let total_visits = page_stats_vec.iter().map(|p| p.count).sum();
    let pages = CountRows::from(page_stats_vec);
    referrers_vec.truncate(5);
    let referrers = CountRows::from(referrers_vec);

    Ok(HxStats {
        base_url: state.base_url,
        pages,
        referrers,
        chart,
        range,
        total_visits,
        path: q.path,
    })
}
