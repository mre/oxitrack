use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use futures::{StreamExt, TryStreamExt};
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status};
use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use tracing::instrument;

use crate::db::{Id, TimeStamp};

use super::{states::AppState, AppStateT};

async fn handle_history(state: Arc<AppState>, path: &str) -> Result<Json<Vec<String>>, RespErr> {
    let path_id = sqlx::query_as!(Id, "SELECT id FROM paths WHERE path = $1", path)
        .fetch_one(&*state.db)
        .await
        .ctx(Status::NotFound)
        .err_msg("Path not found!")?
        .id;

    sqlx::query_as!(
        TimeStamp,
        "SELECT timestamp FROM calls WHERE path_id = $1 ORDER BY timestamp",
        path_id,
    )
    .fetch(&*state.db)
    .map(|row| {
        row.ctx(Status::Internal)
            .err_msg("History query failed!")?
            .timestamp
            .to_offset(state.utc_offset)
            .format(&Rfc3339)
            .ctx(Status::Internal)
            .err_msg("Failed to format datetime!")
    })
    .try_collect()
    .await
    .map(Json)
}

#[instrument(skip_all)]
pub async fn history_index(State(state): AppStateT) -> Result<Json<Vec<String>>, RespErr> {
    let path = "";

    handle_history(state, path).await
}

#[instrument(skip_all)]
pub async fn history(
    State(state): AppStateT,
    Path(path): Path<String>,
) -> Result<Json<Vec<String>>, RespErr> {
    let path = path.trim_end_matches('/');

    handle_history(state, path).await
}

#[derive(Serialize)]
pub struct Count {
    path: String,
    count: Option<i64>,
}

#[instrument(skip_all)]
pub async fn counts(State(state): AppStateT) -> Result<Json<Vec<Count>>, RespErr> {
    sqlx::query_as!(
        Count,
        "SELECT path, COUNT(*) AS count FROM calls
        JOIN paths ON paths.id = calls.path_id
        GROUP BY path"
    )
    .fetch_all(&*state.db)
    .await
    .ctx(Status::Internal)
    .err_msg("Counts query failed!")
    .map(Json)
}
