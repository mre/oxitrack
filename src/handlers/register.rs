use axum::{
    extract::{Query, State},
    Json,
};
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status};

use crate::states::AppState;

use super::queries::PathQuery;

pub async fn get(
    State(state): AppState,
    Query(path): Query<PathQuery>,
) -> Result<Json<u16>, RespErr> {
    let path = path.normalized();

    let path_id = sqlx::query!(
        "SELECT id FROM paths
        WHERE path = $1",
        path
    )
    .fetch_optional(&*state.db)
    .await
    .ctx(Status::Internal)
    .err_msg(|| format!("Failed to run path query for path {path}!"))?;

    let path_id = if let Some(id) = path_id {
        id.id
    } else {
        let status = reqwest::get(state.tracked_url_from_path(path))
            .await
            .ctx(Status::NotFound)
            .err_msg(|| format!("Failed to look up the path {path} on the tracked website!"))?
            .status();

        if !status.is_success() {
            return Err(RespErr::new(Status::NotFound)
                .err_msg(format!("Path {path} not found on tracked website!")));
        }

        // There is a possible race condition here.
        // If two requests to the same new path try to insert it at the same time,
        // then only one insertion will be succussful.
        // If the insertion fails because of the constraint, we will try to select.
        let inserted_id = sqlx::query!(
            "INSERT INTO paths(path) VALUES ($1)
                ON CONFLICT ON CONSTRAINT unique_path DO NOTHING
                RETURNING id",
            path
        )
        .fetch_optional(&*state.db)
        .await
        .ctx(Status::Internal)
        .err_msg(|| format!("Failed to insert path {path}!"))?;

        if let Some(id) = inserted_id {
            id.id
        } else {
            // Other request did insert the path first.
            sqlx::query!(
                "SELECT id FROM paths
                        WHERE path = $1",
                path
            )
            .fetch_one(&*state.db)
            .await
            .ctx(Status::Internal)
            .err_msg(|| format!("Failed to insert path {path}!"))?
            .id
        }
    };

    let visitor_id = state.visitor_states.register(path_id);

    Ok(Json(visitor_id))
}
