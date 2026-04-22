use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Query, State};
use axum_ctx::*;
use serde::Deserialize;

use crate::{
    handlers::{
        base_template::Base,
        count_rows::CountRows,
        dashboard::page_stats::{self, PageStat},
        stats_data::{DateRange, build_chart, referrer_count::ReferrerCount},
    },
    states::AppState,
};

#[derive(Deserialize, Default)]
pub struct IndexQuery {
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Template, WebTemplate)]
#[template(path = "index.html")]
pub struct Index {
    pub base: Base<'static>,
    pub base_url: &'static str,
    pub tracked_origin: &'static str,
    pub pages: CountRows<PageStat>,
    pub referrers: CountRows<ReferrerCount>,
    pub chart: Vec<crate::handlers::stats_data::ChartBar>,
    pub range: DateRange,
    pub total_visits: i64,
}

pub async fn get(State(state): AppState, Query(q): Query<IndexQuery>) -> RespResult<Index> {
    let range = DateRange::from_params(q.from, q.to);
    let now = state.now_tz()?;

    let page_stats = page_stats::all_sorted_by_count(state, &range, now).await?;
    let total_visits = page_stats.iter().map(|p| p.count).sum();
    let pages = CountRows::from(page_stats);

    let mut referrers_vec = ReferrerCount::all_sorted_by_count(
        state,
        None,
        range.start_datetime(),
        range.end_datetime(),
    )
    .await?;
    referrers_vec.truncate(5);
    let referrers = CountRows::from(referrers_vec);

    let chart = build_chart(state, None, &range, now).await?;

    Ok(Index {
        base: Base::new(state, "Dashboard"),
        base_url: state.base_url,
        tracked_origin: state.tracked_origin,
        pages,
        referrers,
        chart,
        range,
        total_visits,
    })
}
