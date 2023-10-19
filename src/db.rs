use anyhow::Result;
use axum_ctx::{RespErr, RespErrCtx, RespErrExt, Status};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Serialize)]
pub struct VisitCount {
    pub path: String,
    pub count: i64,
}

impl VisitCount {
    pub async fn all_sorted(pool: &PgPool) -> Result<Vec<Self>, RespErr> {
        sqlx::query_as!(
            Self,
            r#"SELECT path, COUNT(*) AS "count!" FROM paths
            INNER JOIN visits ON visits.path_id = paths.id
            GROUP BY path
            ORDER BY "count!" DESC"#
        )
        .fetch_all(pool)
        .await
        .ctx(Status::Internal)
        .log_msg("Counts query failed!")
    }
}
