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
    pub median_time_spent: Option<SecondsFormatter>,
}

// NULL durations sort last and are excluded from duration_count, but still
// contribute to total_n. The middle rank(s) give the median for odd/even counts.
const VISIT_STATS_QUERY: &str = r"WITH ranked_visits AS (
    SELECT time_s,
        ROW_NUMBER() OVER (ORDER BY time_s IS NULL, time_s) AS duration_rank,
        COUNT(time_s) OVER () AS duration_count
    FROM visits
    WHERE path_id = ?
      AND (? IS NULL OR registered_at >= ?)
      AND (? IS NULL OR registered_at < ?)
)
SELECT COUNT(*) AS total_n,
    AVG(CASE WHEN duration_rank BETWEEN (duration_count + 1) / 2
                                   AND (duration_count + 2) / 2
        THEN time_s END) AS median
FROM ranked_visits";

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

        let stats_row = sqlx::query(VISIT_STATS_QUERY)
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
        let median_time_spent_raw = stats_row.try_get::<Option<f64>, _>("median").ok().flatten();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let median_time_spent = median_time_spent_raw.map(|f| SecondsFormatter(f as u64));

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
            median_time_spent,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_time_spent() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let pool = crate::db::DbPool::connect("sqlite::memory:").await.unwrap();
                sqlx::raw_sql(
                    "CREATE TABLE visits (path_id INTEGER, time_s INTEGER, registered_at TEXT);
                     INSERT INTO visits VALUES
                         (1, 1000, '2026-10-15 00:00:00'),
                         (1, 10, '2026-10-15 00:00:00'),
                         (1, 20, '2026-10-15 00:00:00'),
                         (1, NULL, '2026-10-15 00:00:00'),
                         (1, 9000, '2026-09-30 00:00:00'),
                         (1, 9000, '2026-11-01 00:00:00'),
                         (2, 10, '2026-10-15 00:00:00'),
                         (2, 11, '2026-10-15 00:00:00'),
                         (2, NULL, '2026-10-15 00:00:00'),
                         (3, NULL, '2026-10-15 00:00:00'),
                         (4, 0, '2026-10-15 00:00:00');",
                )
                .execute(&pool)
                .await
                .unwrap();

                for (path_id, count, median) in [
                    (1, 4, Some(20.0)),
                    (2, 3, Some(10.5)),
                    (3, 1, None),
                    (4, 1, Some(0.0)),
                    (5, 0, None),
                ] {
                    let row = sqlx::query(VISIT_STATS_QUERY)
                        .bind(path_id)
                        .bind("2026-10-01 00:00:00")
                        .bind("2026-10-01 00:00:00")
                        .bind("2026-11-01 00:00:00")
                        .bind("2026-11-01 00:00:00")
                        .fetch_one(&pool)
                        .await
                        .unwrap();
                    assert_eq!(row.get::<i64, _>("total_n"), count, "path {path_id}");
                    assert_eq!(
                        row.get::<Option<f64>, _>("median"),
                        median,
                        "path {path_id}"
                    );
                }
            });
    }
}
