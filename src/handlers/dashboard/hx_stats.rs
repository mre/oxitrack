use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Query, State};
use axum_ctx::*;
use serde::Deserialize;

use crate::{
    db::VisitCount,
    handlers::{
        count_rows::CountRows,
        stats_data::{
            Filter, build_chart, chart_width, referrer_count::ReferrerCount,
            start_datetime_for_filter,
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
    pub pages: CountRows<VisitCount>,
    pub referrers: CountRows<ReferrerCount>,
    pub chart: Vec<crate::handlers::stats_data::ChartBar>,
    pub chart_width: f64,
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

    let visits = VisitCount::all_sorted_by_count(state, start_datetime).await?;
    let total_visits = visits.iter().map(|v| v.count).sum();
    let pages = CountRows::from(visits);

    let referrers = ReferrerCount::all_sorted_by_count(state, path_id, start_datetime).await?;
    let referrers = CountRows::from(referrers);

    let chart = build_chart(state, path_id, filter).await?;
    let cw = chart_width(chart.len());

    Ok(HxStats {
        base_url: state.base_url,
        pages,
        referrers,
        chart,
        chart_width: cw,
        filter,
        total_visits,
        path: q.path,
    })
}
