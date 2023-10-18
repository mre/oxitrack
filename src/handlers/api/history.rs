use axum::{
    extract::{Query, State},
    Json,
};
use futures::{StreamExt, TryStreamExt};
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status};
use time::format_description::well_known::Rfc3339;

use crate::{extractors::query_path::QueryPath, states::AppState};

pub async fn get(
    State(state): AppState,
    Query(path): Query<QueryPath>,
) -> Result<Json<Vec<String>>, RespErr> {
    let (path, path_id) = path.normalized_with_id(&state.db).await?;

    sqlx::query!(
        "SELECT registered_at FROM visits
        WHERE path_id = $1
        ORDER BY registered_at",
        path_id,
    )
    .fetch(&*state.db)
    .map(|row| {
        row.ctx(Status::Internal)
            .err_msg(|| format!("History query failed for path {path}!"))?
            .registered_at
            .to_offset(state.utc_offset)
            .format(&Rfc3339)
            .ctx(Status::Internal)
            .err_msg("Failed to format datetime!")
    })
    .try_collect()
    .await
    .map(Json)
}
