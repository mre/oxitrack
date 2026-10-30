use axum_ctx::{RespErrCtx, RespErrExt, RespResult, StatusCode};
use time::PrimitiveDateTime;

use super::local_to_utc;

use crate::{
    db::Db, formatters::SecondsFormatter, handlers::count_rows::Count, states::InnerAppState,
};

pub struct ReferrerCount {
    pub domain: String,
    pub count: i64,
    pub median_duration: Option<SecondsFormatter>,
}

#[derive(sqlx::FromRow)]
struct ReferrerStatRow {
    domain: String,
    count: i64,
    median_time_s: Option<f64>,
}

// Keep unmeasured visits in the count, but rank them last and exclude them
// from the sample size when selecting the middle duration(s).
const REFERRER_STATS_QUERY: &str = r"WITH ranked_visits AS (
    SELECT domain, time_s,
        ROW_NUMBER() OVER (
            PARTITION BY domain ORDER BY time_s IS NULL, time_s
        ) AS duration_rank,
        COUNT(time_s) OVER (PARTITION BY domain) AS duration_count
    FROM visits
    INNER JOIN referrers ON referrers.id = referrer_id
    WHERE (? IS NULL OR path_id = ?)
      AND (? IS NULL OR registered_at >= ?)
      AND (? IS NULL OR registered_at < ?)
)
SELECT domain, COUNT(*) AS count,
    AVG(CASE WHEN duration_rank BETWEEN (duration_count + 1) / 2
                                   AND (duration_count + 2) / 2
        THEN time_s END) AS median_time_s
FROM ranked_visits
GROUP BY domain
ORDER BY count DESC";

impl ReferrerCount {
    pub async fn all_sorted_by_count(
        state: &'static InnerAppState,
        path_id: Option<i64>,
        start_datetime: Option<PrimitiveDateTime>,
        end_datetime: Option<PrimitiveDateTime>,
    ) -> RespResult<Vec<Self>> {
        let start_utc = start_datetime.map(|pdt| local_to_utc(pdt, state.utc_offset));
        let end_utc = end_datetime.map(|pdt| local_to_utc(pdt, state.utc_offset));

        let rows = sqlx::query_as::<Db, ReferrerStatRow>(REFERRER_STATS_QUERY)
            .bind(path_id)
            .bind(path_id)
            .bind(start_utc)
            .bind(start_utc)
            .bind(end_utc)
            .bind(end_utc)
            .fetch_all(&state.pool)
            .await
            .ctx(StatusCode::INTERNAL_SERVER_ERROR)
            .log_msg("Failed to query referrers!")?;

        Ok(rows
            .into_iter()
            .map(|row| Self {
                domain: row.domain,
                count: row.count,
                median_duration: row.median_time_s.map(|s| SecondsFormatter(s as u64)),
            })
            .collect())
    }
}

impl Count for ReferrerCount {
    fn count(&self) -> i64 {
        self.count
    }
}

/// One page a given referrer drives traffic to — the "reverse" view of the
/// referrers table. Produced by [`LinkedPage::all_for_referrer`].
#[derive(sqlx::FromRow)]
pub struct LinkedPage {
    pub path: String,
    pub count: i64,
}

impl Count for LinkedPage {
    fn count(&self) -> i64 {
        self.count
    }
}

impl LinkedPage {
    /// Resolves a referrer `domain` to its id, or `None` if it was never seen.
    pub async fn id_for_domain(
        state: &'static InnerAppState,
        domain: &str,
    ) -> RespResult<Option<i64>> {
        sqlx::query_scalar::<Db, i64>("SELECT id FROM referrers WHERE domain = ? LIMIT 1")
            .bind(domain)
            .fetch_optional(&state.pool)
            .await
            .ctx(StatusCode::INTERNAL_SERVER_ERROR)
            .log_msg("Failed to look up referrer id")
    }

    /// Pages this referrer linked to within the range, most-visited first.
    pub async fn all_for_referrer(
        state: &'static InnerAppState,
        referrer_id: i64,
        start_datetime: Option<PrimitiveDateTime>,
        end_datetime: Option<PrimitiveDateTime>,
    ) -> RespResult<Vec<Self>> {
        let start_utc = start_datetime.map(|pdt| local_to_utc(pdt, state.utc_offset));
        let end_utc = end_datetime.map(|pdt| local_to_utc(pdt, state.utc_offset));

        sqlx::query_as::<Db, Self>(
            r"SELECT paths.path, COUNT(*) AS count FROM visits
            INNER JOIN paths ON paths.id = visits.path_id
            WHERE visits.referrer_id = ?
              AND (? IS NULL OR registered_at >= ?)
              AND (? IS NULL OR registered_at < ?)
            GROUP BY paths.path
            ORDER BY count DESC",
        )
        .bind(referrer_id)
        .bind(start_utc)
        .bind(start_utc)
        .bind(end_utc)
        .bind(end_utc)
        .fetch_all(&state.pool)
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Failed to query pages for referrer!")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_duration_per_referrer() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let pool = crate::db::DbPool::connect("sqlite::memory:").await.unwrap();
                sqlx::migrate!("migrations/sqlite")
                    .run(&pool)
                    .await
                    .unwrap();
                sqlx::raw_sql(
                    "INSERT INTO paths (id, path) VALUES (1, '/post'), (2, '/other');
                     INSERT INTO referrers (id, domain) VALUES
                         (1, 'odd.example'), (2, 'even.example'),
                         (3, 'unknown.example'), (4, 'single.example'), (5, 'empty.example');
                     INSERT INTO visits (path_id, referrer_id, time_s, registered_at) VALUES
                         (1, 1, 10, '2026-10-01 00:00:00'),
                         (1, 1, 20, '2026-10-15 00:00:00'),
                         (1, 1, 1000, '2026-10-15 00:00:00'),
                         (1, 1, NULL, '2026-10-15 00:00:00'),
                         (1, 1, 9000, '2026-09-30 00:00:00'),
                         (1, 1, 9000, '2026-11-01 00:00:00'),
                         (2, 1, 30, '2026-10-15 00:00:00'),
                         (2, 1, 40, '2026-10-15 00:00:00'),
                         (1, 2, 0, '2026-10-15 00:00:00'),
                         (1, 2, 11, '2026-10-15 00:00:00'),
                         (1, 2, NULL, '2026-10-15 00:00:00'),
                         (1, 3, NULL, '2026-10-15 00:00:00'),
                         (1, 4, 42, '2026-10-15 00:00:00'),
                         (1, NULL, 1, '2026-10-15 00:00:00');",
                )
                .execute(&pool)
                .await
                .unwrap();

                for (path_id, start, end, odd_count, odd_median) in [
                    (
                        Some(1),
                        Some("2026-10-01 00:00:00"),
                        Some("2026-11-01 00:00:00"),
                        4,
                        20.0,
                    ),
                    (
                        None,
                        Some("2026-10-01 00:00:00"),
                        Some("2026-11-01 00:00:00"),
                        6,
                        30.0,
                    ),
                    (Some(1), None, None, 6, 1000.0),
                ] {
                    let rows = sqlx::query_as::<Db, ReferrerStatRow>(REFERRER_STATS_QUERY)
                        .bind(path_id)
                        .bind(path_id)
                        .bind(start)
                        .bind(start)
                        .bind(end)
                        .bind(end)
                        .fetch_all(&pool)
                        .await
                        .unwrap();

                    assert_eq!(rows.len(), 4);
                    assert_eq!(rows[0].domain, "odd.example");
                    for (domain, count, median) in [
                        ("odd.example", odd_count, Some(odd_median)),
                        ("even.example", 3, Some(5.5)),
                        ("unknown.example", 1, None),
                        ("single.example", 1, Some(42.0)),
                    ] {
                        let row = rows.iter().find(|row| row.domain == domain).unwrap();
                        assert_eq!(row.count, count, "{domain}");
                        assert_eq!(row.median_time_s, median, "{domain}");
                    }
                }

                let rows = sqlx::query_as::<Db, ReferrerStatRow>(REFERRER_STATS_QUERY)
                    .bind(99)
                    .bind(99)
                    .bind(None::<&str>)
                    .bind(None::<&str>)
                    .bind(None::<&str>)
                    .bind(None::<&str>)
                    .fetch_all(&pool)
                    .await
                    .unwrap();
                assert!(rows.is_empty());
            });
    }
}
