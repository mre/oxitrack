use axum::{
    extract::{Query, State},
    Json,
};
use futures::{StreamExt, TryStreamExt};
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status};
use time::format_description::well_known::Rfc3339;

use crate::{
    db::{Id, TimeStamp},
    handlers::queries::PathQuery,
    states::AppState,
};

pub async fn get(
    State(state): AppState,
    Query(path): Query<PathQuery>,
) -> Result<Json<Vec<String>>, RespErr> {
    let path = path.normalized();

    let path_id = sqlx::query_as!(
        Id,
        "SELECT id FROM paths
        WHERE path = $1",
        path
    )
    .fetch_one(&*state.db)
    .await
    .ctx(Status::NotFound)
    .err_msg_lz(|| format!("Path {path} not found!"))?
    .id;

    sqlx::query_as!(
        TimeStamp,
        "SELECT timestamp FROM visits
        WHERE path_id = $1
        ORDER BY timestamp",
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
