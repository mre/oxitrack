use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Query, State};
use axum_ctx::RespResult;
use time::Duration;

use crate::{
    handlers::{
        base_template::Base,
        count_rows::CountRows,
        dashboard::page_stats::{self, PageStat},
        stats_data::{
            DateRange, PresetButton, StatsLink, build_chart, referrer_count::ReferrerCount,
        },
    },
    states::AppState,
};

#[derive(Template, WebTemplate)]
#[template(path = "index.html")]
pub struct Index {
    pub base: Base<'static>,
    pub tracked_origin: &'static str,
    pub pages: CountRows<PageStat>,
    pub referrers: CountRows<ReferrerCount>,
    pub chart: Vec<crate::handlers::stats_data::ChartBar>,
    pub range: DateRange,
    pub total_visits: i64,
    /// Filter buttons rendered server-side so the template stays logic-free.
    pub preset_buttons: Vec<PresetButton>,
    /// URL the live indicator should poll; carries the active range so the
    /// totals stay in sync with the current filter without any client-side glue.
    pub live_url: String,
}

pub async fn get(State(state): AppState, Query(range): Query<DateRange>) -> RespResult<Index> {
    let now = state.now_tz()?;
    let range = if range.from.is_none() && range.to.is_none() {
        let to = now.date();
        let from = to - Duration::days(90);
        DateRange {
            from: Some(from),
            to: Some(to),
        }
    } else {
        range
    };

    let (page_stats_vec, mut referrers_vec, chart) = tokio::try_join!(
        page_stats::all_sorted_by_count(state, &range, now),
        ReferrerCount::all_sorted_by_count(
            state,
            None,
            range.start_datetime(),
            range.end_datetime()
        ),
        build_chart(state, None, &range, now),
    )?;

    let total_visits = page_stats_vec.iter().map(|p| p.count).sum();
    let pages = CountRows::from(page_stats_vec);
    referrers_vec.truncate(5);
    let referrers = CountRows::from(referrers_vec);

    let preset_buttons = StatsLink::preset_buttons("/hx/stats", None, now.date(), &range);
    let live_url = StatsLink::new(&range, None).url("/api/live");

    Ok(Index {
        base: Base::new(state, "Dashboard"),
        tracked_origin: state.tracked_origin,
        pages,
        referrers,
        chart,
        range,
        total_visits,
        preset_buttons,
        live_url,
    })
}
