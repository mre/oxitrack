use axum::{
    extract::{Query, State},
    Json,
};
use futures::{StreamExt, TryStreamExt};
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status};
use serde::Serialize;
use time::format_description::well_known::Rfc3339;
use tracing::instrument;

use crate::db::{Id, TimeStamp};

use super::{AppStateT, PathQuery};

#[instrument(skip_all)]
pub async fn history(
    State(state): AppStateT,
    Query(PathQuery { path }): Query<PathQuery>,
) -> Result<Json<Vec<String>>, RespErr> {
    let path = path.trim_end_matches('/');

    let path_id = sqlx::query_as!(Id, "SELECT id FROM paths WHERE path = $1", path)
        .fetch_one(&*state.db)
        .await
        .ctx(Status::NotFound)
        .err_msg_lz(|| format!("Path {path} not found!"))?
        .id;

    sqlx::query_as!(
        TimeStamp,
        "SELECT timestamp FROM calls WHERE path_id = $1 ORDER BY timestamp",
        path_id,
    )
    .fetch(&*state.db)
    .map(|row| {
        row.ctx(Status::Internal)
            .err_msg_lz(|| format!("History query failed for path {path}!"))?
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
