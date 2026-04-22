use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Query, State};
use axum_ctx::*;
use serde::Deserialize;

use crate::{
    handlers::{
        count_rows::CountRows,
        dashboard::page_stats::{self, PageStat},
        stats_data::{
            Filter, build_chart, referrer_count::ReferrerCount, start_datetime_for_filter,
        },
    },
    states::AppState,
};

#[derive(Deserialize, Default)]
pub struct HxStatsQuery {
    #[serde(default)]
    pub filter: Filter,
    pub path: Option<String>,
}

#[derive(Template, WebTemplate)]
#[template(path = "hx_stats.html")]
pub struct HxStats {
    pub base_url: &'static str,
    pub pages: CountRows<PageStat>,
    pub referrers: CountRows<ReferrerCount>,
    pub chart: Vec<crate::handlers::stats_data::ChartBar>,
    pub filter: Filter,
    pub total_visits: i64,
    pub path: Option<String>,
}

pub async fn get(State(state): AppState, Query(q): Query<HxStatsQuery>) -> RespResult<HxStats> {
    let filter = q.filter;
    let now = state.now_tz()?;
    let start_datetime = start_datetime_for_filter(filter, now)?;

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

    let page_stats = page_stats::all_sorted_by_count(state, filter, now, start_datetime).await?;
    let total_visits = page_stats.iter().map(|p| p.count).sum();
    let pages = CountRows::from(page_stats);

    let referrers = ReferrerCount::all_sorted_by_count(state, path_id, start_datetime).await?;
    let referrers = CountRows::from(referrers);

    let chart = build_chart(state, path_id, filter).await?;

    Ok(HxStats {
        base_url: state.base_url,
        pages,
        referrers,
        chart,
        filter,
        total_visits,
        path: q.path,
    })
}
