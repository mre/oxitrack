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
        stats_data::{
            Filter, build_chart, referrer_count::ReferrerCount, start_datetime_for_filter,
        },
    },
    states::AppState,
};

#[derive(Deserialize, Default)]
pub struct IndexQuery {
    #[serde(default)]
    pub filter: Filter,
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
    pub filter: Filter,
    pub total_visits: i64,
}

pub async fn get(State(state): AppState, Query(q): Query<IndexQuery>) -> RespResult<Index> {
    let filter = q.filter;
    let now = state.now_tz()?;
    let start_datetime = start_datetime_for_filter(filter, now)?;

    let page_stats = page_stats::all_sorted_by_count(state, filter, now, start_datetime).await?;
    let total_visits = page_stats.iter().map(|p| p.count).sum();
    let pages = CountRows::from(page_stats);

    let mut referrers_vec = ReferrerCount::all_sorted_by_count(state, None, start_datetime).await?;
    referrers_vec.truncate(5);
    let referrers = CountRows::from(referrers_vec);

    let chart = build_chart(state, None, filter).await?;

    Ok(Index {
        base: Base::new(state, "Dashboard"),
        base_url: state.base_url,
        tracked_origin: state.tracked_origin,
        pages,
        referrers,
        chart,
        filter,
        total_visits,
    })
}
