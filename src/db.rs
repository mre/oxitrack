use axum_ctx::*;
use serde::Serialize;
use time::PrimitiveDateTime;

use crate::{handlers::count_rows::Count, states::InnerAppState};

pub type DbPool = sqlx::SqlitePool;
pub type DbConnection = sqlx::SqliteConnection;
pub type Db = sqlx::Sqlite;

#[derive(Serialize, sqlx::FromRow)]
pub struct VisitCount {
    pub path: String,
    pub count: i64,
}

impl VisitCount {
    pub async fn all_sorted_by_count(
        state: &'static InnerAppState,
        start_datetime: Option<PrimitiveDateTime>,
    ) -> RespResult<Vec<Self>> {
        sqlx::query_as::<_, Self>(
            r#"SELECT path, COUNT(*) AS count FROM paths
            INNER JOIN visits ON visits.path_id = paths.id
            WHERE ? IS NULL OR datetime(registered_at, ?) >= datetime(?)
            GROUP BY path
            ORDER BY count DESC"#,
        )
        .bind(start_datetime)
        .bind(state.posix_utc_offset_str)
        .bind(start_datetime)
        .fetch_all(&state.pool)
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Counts query failed!")
    }
}

impl Count for VisitCount {
    #[inline]
    fn count(&self) -> i64 {
        self.count
    }
}
