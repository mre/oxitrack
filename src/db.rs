use axum_ctx::*;
use serde::Serialize;

use time::PrimitiveDateTime;

use crate::{handlers::count_rows::Count, states::InnerAppState};

// ---------------------------------------------------------------------------
// Database type aliases – switch between Postgres and SQLite via features.
// Use `--no-default-features --features sqlite` to build with SQLite.
// ---------------------------------------------------------------------------

#[cfg(feature = "postgres")]
pub type DbPool = sqlx::PgPool;
#[cfg(feature = "sqlite")]
pub type DbPool = sqlx::SqlitePool;

#[cfg(feature = "postgres")]
pub type DbConnection = sqlx::PgConnection;
#[cfg(feature = "sqlite")]
pub type DbConnection = sqlx::SqliteConnection;

/// Marker type used as the `DB` type parameter in `sqlx::query_as::<Db, _>`.
#[cfg(feature = "postgres")]
pub type Db = sqlx::Postgres;
#[cfg(feature = "sqlite")]
pub type Db = sqlx::Sqlite;

// ---------------------------------------------------------------------------

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
        // PostgreSQL: $1 used twice (positional), so only 2 bindings needed.
        #[cfg(feature = "postgres")]
        let result = sqlx::query_as::<_, Self>(
            r#"SELECT path, COUNT(*) AS count FROM paths
            INNER JOIN visits ON visits.path_id = paths.id
            WHERE $1 IS NULL OR TIMEZONE($2, registered_at) >= $1
            GROUP BY path
            ORDER BY count DESC"#,
        )
        .bind(start_datetime)
        .bind(state.posix_utc_offset_str)
        .fetch_all(&state.pool)
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Counts query failed!");

        // SQLite: each `?` is a separate positional slot, so duplicate bindings.
        #[cfg(feature = "sqlite")]
        let result = sqlx::query_as::<_, Self>(
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
        .log_msg("Counts query failed!");

        result
    }
}

impl Count for VisitCount {
    #[inline]
    fn count(&self) -> i64 {
        self.count
    }
}
