use axum::{
    extract::{Query, State},
    Json,
};
use axum_ctx::{RespErr, RespErrCtx, RespErrExt, Status};
use time::OffsetDateTime;

use crate::{
    extractors::query_path::QueryPath,
    states::{visitor_state::SleepingState, AppState},
};

pub async fn get(
    State(state): AppState,
    Query(path): Query<QueryPath>,
) -> Result<Json<u16>, RespErr> {
    // As early as possible for a correct time measurement.
    let registered_at = OffsetDateTime::now_utc();

    let path = path.normalized();

    let path_id = sqlx::query!(
        "SELECT id FROM paths
        WHERE path = $1",
        path
    )
    .fetch_optional(&state.pool)
    .await
    .ctx(Status::Internal)
    .log_msg(|| format!("Failed to run path query for path {path}!"))?;

    let path_id = if let Some(id) = path_id {
        id.id
    } else {
        let status = state
            .http_client
            .get(state.tracked_url_from_path(path))
            .send()
            .await
            .ctx(Status::NotFound)
            .user_msg(|| format!("Failed to look up the path {path} on the tracked website!"))?
            .status();

        if !status.is_success() {
            return Err(RespErr::new(Status::NotFound)
                .log_msg(format!("Path {path} not found on tracked website!")));
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
        .fetch_optional(&state.pool)
        .await
        .ctx(Status::Internal)
        .log_msg(|| format!("Failed to insert path {path}!"))?;

        if let Some(id) = inserted_id {
            id.id
        } else {
            // Other request did insert the path first.
            sqlx::query!(
                "SELECT id FROM paths
                WHERE path = $1",
                path
            )
            .fetch_one(&state.pool)
            .await
            .ctx(Status::Internal)
            .log_msg(|| format!("Failed to insert path {path}!"))?
            .id
        }
    };

    let visitor_id = state.visitor_states.register(SleepingState {
        path_id,
        registered_at,
    });

    Ok(Json(visitor_id))
}
