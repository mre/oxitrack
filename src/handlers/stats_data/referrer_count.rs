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
        path_id: i64,
        start_datetime: Option<PrimitiveDateTime>,
    ) -> RespResult<Vec<Self>> {
        // PostgreSQL: $2 is used twice (positional reuse), so only 3 bindings needed.
        #[cfg(feature = "postgres")]
        let result = sqlx::query_as::<Db, Self>(
            r#"SELECT domain, COUNT(*) AS count FROM visits
            INNER JOIN referrers ON referrers.id = referrer_id
            WHERE path_id = $1 AND ($2 IS NULL OR TIMEZONE($3, registered_at) >= $2)
            GROUP BY domain
            ORDER BY count DESC"#,
        )
        .bind(path_id)
        .bind(start_datetime)
        .bind(state.posix_utc_offset_str)
        .fetch_all(&state.pool)
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Failed to query referrers!");

        // SQLite: each `?` is a separate slot, so start_datetime is bound twice.
        #[cfg(feature = "sqlite")]
        let result = sqlx::query_as::<Db, Self>(
            r#"SELECT domain, COUNT(*) AS count FROM visits
            INNER JOIN referrers ON referrers.id = referrer_id
            WHERE path_id = ? AND (? IS NULL OR datetime(registered_at, ?) >= datetime(?))
            GROUP BY domain
            ORDER BY count DESC"#,
        )
        .bind(path_id)
        .bind(start_datetime)
        .bind(state.posix_utc_offset_str)
        .bind(start_datetime)
        .fetch_all(&state.pool)
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Failed to query referrers!");

        result
    }
}

impl Count for ReferrerCount {
    fn count(&self) -> i64 {
        self.count
    }
}
