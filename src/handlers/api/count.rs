use axum::{
    Json,
    extract::{Query, State},
};
use axum_ctx::*;
use sqlx::Row;

use crate::{extractors::query_path::QueryPath, states::AppState};

pub async fn get(State(state): AppState, Query(path): Query<QueryPath>) -> RespResult<Json<i64>> {
    let path = path.normalized();

    #[cfg(feature = "postgres")]
    let sql = r#"SELECT COUNT(*) AS count FROM visits
        JOIN paths ON paths.id = path_id
        WHERE path = $1"#;

    #[cfg(feature = "sqlite")]
    let sql = r#"SELECT COUNT(*) AS count FROM visits
        JOIN paths ON paths.id = path_id
        WHERE path = ?"#;

    sqlx::query(sql)
        .bind(path)
        .fetch_one(&state.pool)
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Failed to query the count of visits.")
        .map(|row| Json(row.get::<i64, _>("count")))
}
