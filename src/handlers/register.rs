use axum::{
    extract::{Query, State},
    Json,
};
use axum_ctx::*;
use time::OffsetDateTime;

use crate::{
    extractors::query_path::QueryPath,
    states::{
        visitor_state::{SleepingState, VisitorId},
        AppState,
    },
};

pub async fn get(
    State(state): AppState,
    Query(path): Query<QueryPath>,
) -> Result<Json<VisitorId>, RespErr> {
    // As early as possible for a correct time measurement.
    let registered_at = OffsetDateTime::now_utc();

    let path = path.normalized();

    let path_row = sqlx::query!(
        "SELECT id FROM paths
        WHERE path = $1
        LIMIT 1",
        path,
    )
    .fetch_optional(&state.pool)
    .await
    .ctx(StatusCode::INTERNAL_SERVER_ERROR)
    .log_msg(|| format!("Failed to run the path query for the path {path}!"))?;

    let path_id = if let Some(row) = path_row {
        row.id
    } else {
        let status = state
            .http_client
            .get(state.tracked_url_from_path(path))
            .send()
            .await
            .ctx(StatusCode::NOT_FOUND)
            .user_msg(|| format!("Failed to look up the path {path} on the tracked website!"))?
            .status();

        if !status.is_success() {
            return Err(RespErr::new(StatusCode::NOT_FOUND).user_msg(format!(
                "The path {path} was not found on the tracked website!"
            )));
        }

        // There is a possible race condition here.
        // If two requests try to insert at the same time,
        // then only one insertion will be succussful.
        // If the insertion fails because of the constraint, we will try to select.
        let inserted_row = sqlx::query!(
            "INSERT INTO paths(path)
            VALUES ($1)
            ON CONFLICT ON CONSTRAINT unique_path DO NOTHING
            RETURNING id",
            path,
        )
        .fetch_optional(&state.pool)
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg(|| format!("Failed to insert the path {path}!"))?;

        if let Some(row) = inserted_row {
            row.id
        } else {
            // A concurrent request inserted first.
            sqlx::query!(
                "SELECT id FROM paths
                WHERE path = $1
                LIMIT 1",
                path,
            )
            .fetch_one(&state.pool)
            .await
            .ctx(StatusCode::INTERNAL_SERVER_ERROR)
            .log_msg(|| format!("Failed to insert the path {path}!"))?
            .id
        }
    };

    let visitor_id = state.visitor_states.register(SleepingState {
        path_id,
        registered_at,
    });

    Ok(Json(visitor_id))
}
