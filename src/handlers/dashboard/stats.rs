use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Query, State};
use axum_ctx::{RespErr, RespErrCtx, RespErrExt, RespResult, StatusCode};
use sqlx::Row;

use crate::{
    extractors::query_path::{PathId, QueryPath},
    formatters::{DateTimeVerboseFormatter, SecondsFormatter},
    handlers::{
        base_template::Base,
        count_rows::CountRows,
        stats_data::{
            DateRange, PresetButton, StatsLink, VisitFilter, WholeDaysSinceFirstVisit, build_chart,
            referrer_count::ReferrerCount,
        },
    },
    states::{AppState, InnerAppState},
};

pub struct Visits {
    pub first: DateTimeVerboseFormatter,
    pub total_n: i64,
    pub per_day: f64,
    pub average_time_spent: Option<SecondsFormatter>,
}

impl Visits {
    async fn build(
        state: &'static InnerAppState,
        path_id: i64,
        range: &DateRange,
        now: time::OffsetDateTime,
    ) -> RespResult<Self> {
        let Some(WholeDaysSinceFirstVisit {
            whole_days_since_first_visit,
            first_visit,
        }) = WholeDaysSinceFirstVisit::build(state, VisitFilter::path(path_id), now).await?
        else {
            return Err(RespErr::new(StatusCode::NOT_FOUND)
                .user_msg("The requested path has no counted visits yet."));
        };

        let stats_row = sqlx::query(
            r"SELECT COUNT(*) AS total_n, AVG(time_s) AS avg
              FROM visits
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
        .log_msg("Failed to query visit stats!")?;

        let total_n_visits: i64 = stats_row.get("total_n");
        let average_time_spent_raw = stats_row.try_get::<Option<f64>, _>("avg").ok().flatten();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let average_time_spent = average_time_spent_raw.map(|f| SecondsFormatter(f as u64));

        let first_visit = state.apply_utc_offset(first_visit)?;

        #[allow(clippy::cast_precision_loss)]
        let days = range
            .whole_days_since_first_visit(first_visit.date(), now)
            .min(whole_days_since_first_visit.max(1));
        let visits_per_day = if days > 0 {
            total_n_visits as f64 / days as f64
        } else {
            total_n_visits as f64
        };

        Ok(Self {
            first: DateTimeVerboseFormatter(first_visit),
            total_n: total_n_visits,
            per_day: visits_per_day,
            average_time_spent,
        })
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "stats.html")]
pub struct Stats {
    pub base: Base,
    pub tracked_origin: &'static str,
    pub path: String,
    pub visits: Visits,
    pub referrers: CountRows<ReferrerCount>,
    pub chart: Vec<crate::handlers::stats_data::ChartBar>,
    pub range: DateRange,
    /// Filter buttons rendered server-side so the template stays logic-free.
    pub preset_buttons: Vec<PresetButton>,
    /// URL chart selections should refresh, with the path filter preserved.
    pub zoom_url: String,
    /// URL the live indicator should poll; carries the active range and the
    /// current path so totals stay in sync with the current filter without
    /// any client-side glue.
    pub live_url: String,
}

pub async fn get(
    State(state): AppState,
    Query(path_q): Query<QueryPath>,
    Query(range): Query<DateRange>,
) -> RespResult<Stats> {
    let now = state.now_tz()?;
    let range = range.or_last_90_days(now);

    let PathId { path, path_id } = path_q.normalized_with_id(&state.pool).await?;

    let visits = Visits::build(state, path_id, &range, now).await?;

    let referrers = ReferrerCount::all_sorted_by_count(
        state,
        Some(path_id),
        range.start_datetime(),
        range.end_datetime(),
    )
    .await?;
    let referrers = CountRows::from(referrers);

    let chart = build_chart(state, VisitFilter::path(path_id), &range, now).await?;

    let stats_link = StatsLink::new(&range, Some(path));
    let preset_buttons = stats_link.preset_buttons("/hx/stats", now.date());
    let zoom_url = stats_link.url("/hx/stats");
    let live_url = stats_link.url("/api/live");

    Ok(Stats {
        base: Base::new(state, path),
        tracked_origin: state.tracked_origin,
        path: path.to_owned(),
        visits,
        referrers,
        chart,
        range,
        preset_buttons,
        zoom_url,
        live_url,
    })
}
