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
            DateRange, PanelView, PresetButton, StatsLink, ViewQuery, VisitFilter, build_chart,
            referrer_count::ReferrerCount,
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
    /// Whether the Referrers tab is the active one (Pages otherwise).
    pub is_referrers: bool,
    /// hx-get URLs for the Pages / Referrers tab buttons.
    pub pages_tab_url: String,
    pub referrers_tab_url: String,
    /// hx-get base URLs for the active-search filter inputs (htmx appends `q`).
    pub pages_search_url: String,
    pub referrers_search_url: String,
    /// Filter buttons rendered server-side so the template stays logic-free.
    pub preset_buttons: Vec<PresetButton>,
    /// URL the live indicator should poll; carries the active range so the
    /// totals stay in sync with the current filter without any client-side glue.
    pub live_url: String,
}

pub async fn get(
    State(state): AppState,
    Query(range): Query<DateRange>,
    Query(view_q): Query<ViewQuery>,
) -> RespResult<Index> {
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
    let view = view_q.panel();

    let (page_stats_vec, referrers_vec, chart) = tokio::try_join!(
        page_stats::all_sorted_by_count(state, &range, now),
        ReferrerCount::all_sorted_by_count(
            state,
            None,
            range.start_datetime(),
            range.end_datetime()
        ),
        build_chart(state, VisitFilter::default(), &range, now),
    )?;

    let total_visits = page_stats_vec.iter().map(|p| p.count).sum();
    let pages = CountRows::from(page_stats_vec);
    let referrers = CountRows::from(referrers_vec);

    let preset_buttons = StatsLink::new(&range, None)
        .with_view(view)
        .preset_buttons("/hx/stats", now.date());
    let live_url = StatsLink::new(&range, None).url("/api/live");
    let pages_tab_url = StatsLink::new(&range, None)
        .with_view(PanelView::Pages)
        .url("/hx/stats");
    let referrers_tab_url = StatsLink::new(&range, None)
        .with_view(PanelView::Referrers)
        .url("/hx/stats");
    let pages_search_url = StatsLink::new(&range, None).url("/hx/pages");
    let referrers_search_url = StatsLink::new(&range, None).url("/hx/referrers");

    Ok(Index {
        base: Base::new(state, "Dashboard"),
        tracked_origin: state.tracked_origin,
        pages,
        referrers,
        chart,
        range,
        total_visits,
        is_referrers: view.is_referrers(),
        pages_tab_url,
        referrers_tab_url,
        pages_search_url,
        referrers_search_url,
        preset_buttons,
        live_url,
    })
}
