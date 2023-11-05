use axum::{
    extract::{Query, State},
    Json,
};
use axum_ctx::{RespErr, RespErrCtx, RespErrExt, Status};

use crate::{extractors::query_path::QueryPath, states::AppState};

pub async fn get(
    State(state): AppState,
    Query(path): Query<QueryPath>,
) -> Result<Json<i64>, RespErr> {
    let path = path.normalized();

    sqlx::query!(
        r#"SELECT COUNT(*) AS "count!" FROM visits
        JOIN paths ON paths.id = path_id
        WHERE path = $1"#,
        path,
    )
    .fetch_one(&state.pool)
    .await
    .ctx(Status::Internal)
    .log_msg("Failed to query the count of visits.")
    .map(|row| Json(row.count))
}
