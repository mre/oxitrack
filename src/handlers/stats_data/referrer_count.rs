use axum_ctx::{RespErrCtx, RespErrExt, RespResult, StatusCode};
use time::PrimitiveDateTime;

use super::local_to_utc;

use crate::{db::Db, handlers::count_rows::Count, states::InnerAppState};

#[derive(sqlx::FromRow)]
pub struct ReferrerCount {
    pub domain: String,
    pub count: i64,
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
            r"SELECT domain, COUNT(*) AS count FROM visits
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
