use axum_ctx::{RespErrCtx, RespErrExt, RespResult, StatusCode};
use time::PrimitiveDateTime;

use super::local_to_utc;

use crate::{db::Db, handlers::count_rows::Count, states::InnerAppState};

#[derive(sqlx::FromRow)]
pub struct ReferrerCount {
    pub domain: String,
    pub count: i64,
    pub pages: i64,
}

impl ReferrerCount {
    pub async fn all_sorted_by_count(
        state: &'static InnerAppState,
        path_id: Option<i64>,
        start_datetime: Option<PrimitiveDateTime>,
        end_datetime: Option<PrimitiveDateTime>,
    ) -> RespResult<Vec<Self>> {
        let start_utc = start_datetime.map(|pdt| local_to_utc(pdt, state.utc_offset));
        let end_utc = end_datetime.map(|pdt| local_to_utc(pdt, state.utc_offset));

        sqlx::query_as::<Db, Self>(
            r"SELECT domain, COUNT(*) AS count, COUNT(DISTINCT path_id) AS pages FROM visits
            INNER JOIN referrers ON referrers.id = referrer_id
            WHERE (? IS NULL OR path_id = ?)
              AND (? IS NULL OR registered_at >= ?)
              AND (? IS NULL OR registered_at < ?)
            GROUP BY domain
            ORDER BY count DESC",
        )
        .bind(path_id)
        .bind(path_id)
        .bind(start_utc)
        .bind(start_utc)
        .bind(end_utc)
        .bind(end_utc)
        .fetch_all(&state.pool)
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Failed to query referrers!")
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
