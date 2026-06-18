use askama::Template;
use askama_web::WebTemplate;
use axum::{
    extract::{Query, State},
    http::{HeaderName, HeaderValue},
    response::IntoResponse,
};
use axum_ctx::{RespErrCtx, RespErrExt, RespResult, StatusCode};
use serde::Deserialize;

use crate::{
    handlers::{
        count_rows::CountRows,
        dashboard::page_stats::{self, PageStat},
        stats_data::{
            DateRange, PanelView, PresetButton, StatsLink, ViewQuery, VisitFilter, build_chart,
            referrer_count::ReferrerCount,
        },
    },
    states::AppState,
};

/// Path is the only hx-stats specific query parameter; the date range and tab
/// view are parsed separately via the shared [`DateRange`] / [`ViewQuery`]
/// extractors so that adding more filter params later requires no changes here.
#[derive(Deserialize, Default)]
pub struct PathQuery {
    pub path: Option<String>,
}

#[derive(Template, WebTemplate)]
#[template(path = "hx_stats.html")]
pub struct HxStats {
    pub pages: CountRows<PageStat>,
    pub referrers: CountRows<ReferrerCount>,
    pub chart: Vec<crate::handlers::stats_data::ChartBar>,
    pub range: DateRange,
    pub total_visits: i64,
    pub path: Option<String>,
    /// Dashboard tab state (ignored on the per-path subpage view).
    pub is_referrers: bool,
    pub pages_tab_url: String,
    pub referrers_tab_url: String,
    pub pages_search_url: String,
    pub referrers_search_url: String,
    pub preset_buttons: Vec<PresetButton>,
    /// URL the live indicator should poll, with the active range baked in.
    pub live_url: String,
}

/// `HX-Push-Url` tells htmx to update `window.location` to this URL after the
/// swap, so the browser address bar reflects the active filter without any
/// custom JS. See <https://htmx.org/headers/hx-push-url/>.
const HX_PUSH_URL: HeaderName = HeaderName::from_static("hx-push-url");

#[allow(clippy::too_many_lines)]
pub async fn get(
    State(state): AppState,
    Query(range): Query<DateRange>,
    Query(q): Query<PathQuery>,
    Query(view_q): Query<ViewQuery>,
) -> RespResult<impl IntoResponse> {
    let now = state.now_tz()?;
    let view = view_q.panel();

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

    let is_subpage = path_id.is_some();

    let (page_stats_vec, mut referrers_vec, chart) = if is_subpage {
        let (referrers, chart) = tokio::try_join!(
            ReferrerCount::all_sorted_by_count(
                state,
                path_id,
                range.start_datetime(),
                range.end_datetime()
            ),
            build_chart(state, VisitFilter::from_path_opt(path_id), &range, now),
        )?;
        (vec![], referrers, chart)
    } else {
        let (pages, referrers, chart) = tokio::try_join!(
            page_stats::all_sorted_by_count(state, &range, now),
            ReferrerCount::all_sorted_by_count(
                state,
                None,
                range.start_datetime(),
                range.end_datetime()
            ),
            build_chart(state, VisitFilter::default(), &range, now),
        )?;
        (pages, referrers, chart)
    };

    let total_visits: i64 = if is_subpage {
        sqlx::query_scalar(
            r"SELECT COUNT(*) FROM visits
              WHERE path_id = ?
                AND (? IS NULL OR registered_at >= ?)
                AND (? IS NULL OR registered_at < ?)",
        )
        .bind(path_id)
        .bind(range.start_datetime())
        .bind(range.start_datetime())
        .bind(range.end_datetime())
        .bind(range.end_datetime())
        .fetch_one(&state.pool)
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Failed to query total visits for path")?
    } else {
        page_stats_vec.iter().map(|p| p.count).sum()
    };

    let pages = if is_subpage {
        CountRows::from(vec![])
    } else {
        CountRows::from(page_stats_vec)
    };
    // The per-path subpage only previews its top referrers; the dashboard's
    // Referrers tab shows the full list.
    if is_subpage {
        referrers_vec.truncate(5);
    }
    let referrers = CountRows::from(referrers_vec);

    let path_for_links = q.path.as_deref();

    // Each preset button hits `/hx/stats` with the new range and the current
    // path / tab (if any), so the server keeps producing the right view on click.
    let preset_buttons = StatsLink::new(&range, path_for_links)
        .with_view(view)
        .preset_buttons("/hx/stats", now.date());

    let pages_tab_url = StatsLink::new(&range, None)
        .with_view(PanelView::Pages)
        .url("/hx/stats");
    let referrers_tab_url = StatsLink::new(&range, None)
        .with_view(PanelView::Referrers)
        .url("/hx/stats");
    let pages_search_url = StatsLink::new(&range, None).url("/hx/pages");
    let referrers_search_url = StatsLink::new(&range, None).url("/hx/referrers");

    // Live indicator polls the same range as the current view so the totals
    // stay in sync; passing the range via the URL means no client-side glue.
    let live_url = StatsLink::new(&range, path_for_links).url("/api/live");

    // Public URL the user should see in the address bar after the swap.
    // Sub-pages render at `/stats?...`; otherwise we land on `/` (carrying the
    // active tab so a referrers-tab swap is bookmarkable / reloadable).
    let push_url = if is_subpage {
        StatsLink::new(&range, path_for_links).url("/stats")
    } else {
        StatsLink::new(&range, None).with_view(view).url("/")
    };

    let body = HxStats {
        pages,
        referrers,
        chart,
        range,
        total_visits,
        path: q.path,
        is_referrers: view.is_referrers(),
        pages_tab_url,
        referrers_tab_url,
        pages_search_url,
        referrers_search_url,
        preset_buttons,
        live_url,
    };

    let push_header = HeaderValue::try_from(push_url)
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Failed to build HX-Push-Url header")?;

    Ok(([(HX_PUSH_URL, push_header)], body))
}
