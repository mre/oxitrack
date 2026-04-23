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
            DateRange, PresetButton, StatsLink, build_chart, referrer_count::ReferrerCount,
        },
    },
    states::AppState,
};

/// Path is the only hx-stats specific query parameter; the date range is parsed
/// separately via the shared [`DateRange`] extractor so that adding more
/// filter params later requires no changes here.
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
    pub preset_buttons: Vec<PresetButton>,
    /// URL the live indicator should poll, with the active range baked in.
    pub live_url: String,
}

/// `HX-Push-Url` tells htmx to update `window.location` to this URL after the
/// swap, so the browser address bar reflects the active filter without any
/// custom JS. See <https://htmx.org/headers/hx-push-url/>.
const HX_PUSH_URL: HeaderName = HeaderName::from_static("hx-push-url");

pub async fn get(
    State(state): AppState,
    Query(range): Query<DateRange>,
    Query(q): Query<PathQuery>,
) -> RespResult<impl IntoResponse> {
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

    let (page_stats_vec, mut referrers_vec, chart) = if path_id.is_some() {
        let (referrers, chart) = tokio::try_join!(
            ReferrerCount::all_sorted_by_count(
                state,
                path_id,
                range.start_datetime(),
                range.end_datetime()
            ),
            build_chart(state, path_id, &range, now),
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
            build_chart(state, None, &range, now),
        )?;
        (pages, referrers, chart)
    };

    let is_subpage = path_id.is_some();

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
    referrers_vec.truncate(5);
    let referrers = CountRows::from(referrers_vec);

    let path_for_links = q.path.as_deref();

    // Each preset button hits `/hx/stats` with the new range and the current
    // path (if any), so the server keeps producing the right view on click.
    let preset_buttons = StatsLink::preset_buttons("/hx/stats", path_for_links, now.date(), &range);

    // Live indicator polls the same range as the current view so the totals
    // stay in sync; passing the range via the URL means no client-side glue.
    let live_url = StatsLink::new(&range, path_for_links).url("/api/live");

    // Public URL the user should see in the address bar after the swap.
    // Sub-pages render at `/stats?...`; otherwise we land on `/`.
    let push_base = if is_subpage { "/stats" } else { "/" };
    let push_url = StatsLink::new(&range, path_for_links).url(push_base);

    let body = HxStats {
        pages,
        referrers,
        chart,
        range,
        total_visits,
        path: q.path,
        preset_buttons,
        live_url,
    };

    let push_header = HeaderValue::try_from(push_url)
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Failed to build HX-Push-Url header")?;

    Ok(([(HX_PUSH_URL, push_header)], body))
}
