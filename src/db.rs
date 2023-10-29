use anyhow::Result;
use axum_ctx::{RespErr, RespErrCtx, RespErrExt, Status};
use serde::Serialize;
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::handlers::count_rows::Count;

#[derive(Serialize)]
pub struct VisitCount {
    pub path: String,
    pub count: i64,
}

impl VisitCount {
    pub async fn all_sorted_by_count(
        pool: &PgPool,
        start_datetime: Option<OffsetDateTime>,
    ) -> Result<Vec<Self>, RespErr> {
        sqlx::query_as!(
            Self,
            r#"SELECT path, COUNT(*) AS "count!" FROM paths
            INNER JOIN visits ON visits.path_id = paths.id
            WHERE $1::timestamptz IS NULL OR registered_at > $1
            GROUP BY path
            ORDER BY "count!" DESC"#,
            start_datetime,
        )
        .fetch_all(pool)
        .await
        .ctx(Status::Internal)
        .log_msg("Counts query failed!")
    }
}

impl Count for VisitCount {
    #[inline]
    fn count(&self) -> i64 {
        self.count
    }
}
