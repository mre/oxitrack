use anyhow::Result;
use axum_ctx::{RespErr, RespErrCtx, RespErrExt, Status};
use serde::Serialize;
use time::PrimitiveDateTime;

use crate::{handlers::count_rows::Count, states::InnerAppState};

#[derive(Serialize)]
pub struct VisitCount {
    pub path: String,
    pub count: i64,
}

impl VisitCount {
    pub async fn all_sorted_by_count(
        state: &'static InnerAppState,
        start_datetime: Option<PrimitiveDateTime>,
    ) -> Result<Vec<Self>, RespErr> {
        sqlx::query_as!(
            Self,
            r#"SELECT path, COUNT(*) AS "count!" FROM paths
            INNER JOIN visits ON visits.path_id = paths.id
            WHERE $1::timestamp IS NULL OR timezone($2, registered_at) >= $1
            GROUP BY path
            ORDER BY "count!" DESC"#,
            start_datetime,
            state.posix_utc_offset_str,
        )
        .fetch_all(&state.pool)
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
