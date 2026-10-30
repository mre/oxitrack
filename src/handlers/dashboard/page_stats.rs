use axum_ctx::{RespErrCtx, RespErrExt, RespResult, StatusCode};
use time::OffsetDateTime;

use crate::{
    db::Db,
    formatters::SecondsFormatter,
    handlers::{
        count_rows::Count,
        stats_data::{DateRange, local_to_utc},
    },
    states::InnerAppState,
};

pub struct PageStat {
    pub path: String,
    pub count: i64,
    pub median_duration: Option<SecondsFormatter>,
    pub per_day: f64,
    pub trend: Trend,
}

/// Direction traffic moved across the active date range, derived by comparing
/// the number of visits in the first half of the range against the second half.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    Up,
    Down,
    Flat,
}

impl Trend {
    /// Arrow glyph shown next to the visits/day value.
    #[inline]
    pub const fn arrow(self) -> &'static str {
        match self {
            Self::Up => "\u{25B2}",   // ▲
            Self::Down => "\u{25BC}", // ▼
            Self::Flat => "",
        }
    }

    /// CSS class used to colour the arrow.
    #[inline]
    pub const fn class(self) -> &'static str {
        match self {
            Self::Up => "trend-up",
            Self::Down => "trend-down",
            Self::Flat => "trend-flat",
        }
    }

    /// Accessible label describing the movement.
    #[inline]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Up => "Traffic grew over this range",
            Self::Down => "Traffic fell over this range",
            Self::Flat => "Traffic held steady over this range",
        }
    }
}

impl Count for PageStat {
    #[inline]
    fn count(&self) -> i64 {
        self.count
    }
}

#[derive(sqlx::FromRow)]
struct PageStatRow {
    path: String,
    count: i64,
    median_time_s: Option<f64>,
    first_registered_at: Option<time::PrimitiveDateTime>,
    first_half: i64,
    second_half: i64,
}

// Rank measured durations first so NULLs do not affect the median, while
// retaining every visit for counts and trends. AVG combines the two middle
// durations for even-sized samples and the single middle duration otherwise.
const PAGE_STATS_QUERY: &str = r"WITH ranked_visits AS (
    SELECT path_id, registered_at, time_s,
        ROW_NUMBER() OVER (
            PARTITION BY path_id ORDER BY time_s IS NULL, time_s
        ) AS duration_rank,
        COUNT(time_s) OVER (PARTITION BY path_id) AS duration_count
    FROM visits
    WHERE (? IS NULL OR registered_at >= ?)
      AND (? IS NULL OR registered_at < ?)
)
SELECT paths.path,
    COUNT(*) AS count,
    AVG(CASE WHEN visits.duration_rank BETWEEN (visits.duration_count + 1) / 2
                                          AND (visits.duration_count + 2) / 2
        THEN visits.time_s END) AS median_time_s,
    (
        SELECT MIN(all_visits.registered_at)
        FROM visits AS all_visits
        WHERE all_visits.path_id = paths.id
    ) AS first_registered_at,
    SUM(CASE WHEN ? IS NOT NULL AND visits.registered_at < ? THEN 1 ELSE 0 END) AS first_half,
    SUM(CASE WHEN ? IS NOT NULL AND visits.registered_at >= ? THEN 1 ELSE 0 END) AS second_half
FROM paths
INNER JOIN ranked_visits AS visits ON visits.path_id = paths.id
GROUP BY paths.path
ORDER BY count DESC";

pub async fn all_sorted_by_count(
    state: &'static InnerAppState,
    range: &DateRange,
    now: OffsetDateTime,
) -> RespResult<Vec<PageStat>> {
    let start_datetime = range.start_datetime();
    let end_datetime = range.end_datetime();

    let start_utc = start_datetime.map(|pdt| local_to_utc(pdt, state.utc_offset));
    let end_utc = end_datetime.map(|pdt| local_to_utc(pdt, state.utc_offset));

    // Split the active range at its midpoint so we can compare visit volume in
    // the earlier half against the later half and derive a per-path trend.
    // Open-ended ranges fall back to "now" for the end; a missing start (e.g.
    // "All time") leaves the midpoint undefined and yields a flat trend.
    let now_utc = local_to_utc(
        time::PrimitiveDateTime::new(now.date(), now.time()),
        state.utc_offset,
    );
    let effective_end = end_utc.unwrap_or(now_utc);
    let midpoint = start_utc.map(|start| start + (effective_end - start) / 2);

    let rows = sqlx::query_as::<Db, PageStatRow>(PAGE_STATS_QUERY)
        .bind(start_utc)
        .bind(start_utc)
        .bind(end_utc)
        .bind(end_utc)
        .bind(midpoint)
        .bind(midpoint)
        .bind(midpoint)
        .bind(midpoint)
        .fetch_all(&state.pool)
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Page stats query failed!")?;

    Ok(rows
        .into_iter()
        .map(|row| {
            #[allow(clippy::cast_precision_loss)]
            let days = row.first_registered_at.map_or_else(
                || range.whole_days(now).unwrap_or(1),
                |fv| range.whole_days_since_first_visit(fv.date(), now),
            ) as f64;
            #[allow(clippy::cast_precision_loss)]
            let per_day = row.count as f64 / days;

            PageStat {
                path: row.path,
                count: row.count,
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                median_duration: row.median_time_s.map(|s| SecondsFormatter(s as u64)),
                per_day,
                trend: match row.second_half.cmp(&row.first_half) {
                    std::cmp::Ordering::Greater => Trend::Up,
                    std::cmp::Ordering::Less => Trend::Down,
                    std::cmp::Ordering::Equal => Trend::Flat,
                },
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_duration_per_path() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let pool = crate::db::DbPool::connect("sqlite::memory:").await.unwrap();
                sqlx::raw_sql(
                    "CREATE TABLE paths (id INTEGER PRIMARY KEY, path TEXT);
                     CREATE TABLE visits (path_id INTEGER, time_s INTEGER, registered_at TEXT);
                     INSERT INTO paths VALUES
                         (1, '/odd'), (2, '/even'), (3, '/unknown'), (4, '/zero'), (5, '/empty');
                     INSERT INTO visits VALUES
                         (1, 1000, '2026-10-15 00:00:00'),
                         (1, 10, '2026-10-15 00:00:00'),
                         (1, 20, '2026-10-16 00:00:00'),
                         (1, NULL, '2026-10-16 00:00:00'),
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

                let rows = sqlx::query_as::<Db, PageStatRow>(PAGE_STATS_QUERY)
                    .bind("2026-10-01 00:00:00")
                    .bind("2026-10-01 00:00:00")
                    .bind("2026-11-01 00:00:00")
                    .bind("2026-11-01 00:00:00")
                    .bind("2026-10-16 00:00:00")
                    .bind("2026-10-16 00:00:00")
                    .bind("2026-10-16 00:00:00")
                    .bind("2026-10-16 00:00:00")
                    .fetch_all(&pool)
                    .await
                    .unwrap();

                assert_eq!(rows.len(), 4);
                for (path, count, median) in [
                    ("/odd", 4, Some(20.0)),
                    ("/even", 3, Some(10.5)),
                    ("/unknown", 1, None),
                    ("/zero", 1, Some(0.0)),
                ] {
                    let row = rows.iter().find(|row| row.path == path).unwrap();
                    assert_eq!(row.count, count, "{path}");
                    assert_eq!(row.median_time_s, median, "{path}");
                }
                let odd = &rows[0];
                assert_eq!(odd.path, "/odd");
                assert_eq!((odd.first_half, odd.second_half), (2, 2));
                assert_eq!(
                    odd.first_registered_at.unwrap().date().to_string(),
                    "2026-09-30"
                );
            });
    }
}
