use axum_ctx::*;
use time::PrimitiveDateTime;

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
    ) -> RespResult<Vec<Self>> {
        sqlx::query_as::<Db, Self>(
            r#"SELECT domain, COUNT(*) AS count FROM visits
            INNER JOIN referrers ON referrers.id = referrer_id
            WHERE (? IS NULL OR path_id = ?) AND (? IS NULL OR datetime(registered_at, ?) >= datetime(?))
            GROUP BY domain
            ORDER BY count DESC"#,
        )
        .bind(path_id)
        .bind(path_id)
        .bind(start_datetime)
        .bind(state.posix_utc_offset_str)
        .bind(start_datetime)
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
