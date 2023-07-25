use axum::{
    extract::{Path, State},
    Json,
};
use futures::TryStreamExt;
use resp_err::{RespErr, RespErrCtx, Status};
use serde::Serialize;
use tracing::instrument;

use crate::db::Id;

use super::AppStateT;

struct TimeStamp {
    timestamp: Option<String>,
}

#[instrument(skip_all)]
pub async fn history(
    State(state): AppStateT,
    Path(path): Path<String>,
) -> Result<Json<Vec<String>>, RespErr> {
    let path_id = sqlx::query_as!(Id, "SELECT id FROM paths WHERE path = $1", path)
        .fetch_one(&*state.db)
        .await
        .ctx(Status::NotFound)?
        .id;

    let timestamps = sqlx::query_as!(
        TimeStamp,
        "SELECT timestamp::text FROM calls WHERE path_id = $1 ORDER BY timestamp",
        path_id,
    )
    .fetch(&*state.db)
    .try_filter_map(|row| async move { Ok(row.timestamp) })
    .try_collect()
    .await
    .ctx(Status::BadRequest)?;

    Ok(Json(timestamps))
}

#[derive(Serialize)]
pub struct Count {
    path: String,
    count: Option<i64>,
}

#[instrument(skip_all)]
pub async fn counts(State(state): AppStateT) -> Result<Json<Vec<Count>>, RespErr> {
    let counts = sqlx::query_as!(
        Count,
        "SELECT path, COUNT(*) AS count FROM calls
        JOIN paths ON paths.id = calls.path_id
        GROUP BY path"
    )
    .fetch_all(&*state.db)
    .await
    .ctx(Status::Internal)?;

    Ok(Json(counts))
}
