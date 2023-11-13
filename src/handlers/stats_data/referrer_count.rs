use axum_ctx::{RespErr, RespErrCtx, RespErrExt, Status};
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::handlers::count_rows::Count;

pub struct ReferrerCount {
    pub domain: String,
    pub count: i64,
}

impl ReferrerCount {
    pub async fn all_sorted_by_count(
        pool: &PgPool,
        path_id: i64,
        start_datetime: Option<OffsetDateTime>,
    ) -> Result<Vec<Self>, RespErr> {
        sqlx::query_as!(
            Self,
            r#"SELECT domain, COUNT(*) as "count!" FROM visits
            INNER JOIN referrers ON referrers.id = referrer_id
            WHERE path_id = $1 AND ($2::timestamptz IS NULL OR registered_at > $2)
            GROUP BY domain
            ORDER BY "count!" DESC"#,
            path_id,
            start_datetime,
        )
        .fetch_all(pool)
        .await
        .ctx(Status::Internal)
        .log_msg("Failed to query referrers!")
    }
}

impl Count for ReferrerCount {
    fn count(&self) -> i64 {
        self.count
    }
}
