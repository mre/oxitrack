use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Query, State};
use axum_ctx::*;
use serde::Deserialize;

use crate::{
    db::VisitCount,
    handlers::{
        base_template::Base,
        count_rows::CountRows,
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
    pub pages: CountRows<VisitCount>,
    pub referrers: CountRows<ReferrerCount>,
    pub chart: Vec<crate::handlers::stats_data::ChartBar>,
    pub filter: Filter,
    pub total_visits: i64,
}

pub async fn get(State(state): AppState, Query(q): Query<IndexQuery>) -> RespResult<Index> {
    let filter = q.filter;
    let now = state.now_tz()?;
    let start_datetime = start_datetime_for_filter(filter, now)?;

    let visits = VisitCount::all_sorted_by_count(state, start_datetime).await?;
    let total_visits = visits.iter().map(|v| v.count).sum();
    let pages = CountRows::from(visits);

    let referrers = ReferrerCount::all_sorted_by_count(state, None, start_datetime).await?;
    let referrers = CountRows::from(referrers);

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
