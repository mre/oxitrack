use axum::{
    Json,
    extract::{Query, State},
};
use axum_ctx::{RespErr, RespErrCtx, RespErrExt, RespResult, StatusCode};
use sqlx::Row;
use time::OffsetDateTime;

use crate::{
    extractors::query_path::QueryPath,
    states::{
        AppState,
        visitor_state::{SleepingState, VisitorId},
    },
};

pub async fn get(
    State(state): AppState,
    Query(path): Query<QueryPath>,
) -> RespResult<Json<VisitorId>> {
    // As early as possible for a correct time measurement.
    let registered_at = OffsetDateTime::now_utc();

    let path = path.normalized();

    let path_row = sqlx::query("SELECT id FROM paths WHERE path = ? LIMIT 1")
        .bind(path)
        .fetch_optional(&state.pool)
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg(|| format!("Failed to run the path query for the path {path}!"))?;

    let path_id = if let Some(row) = path_row {
        row.get::<i64, _>("id")
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
        // then only one insertion will be successful.
        // If the insertion fails because of the constraint, we will try to select.
        let inserted_row = sqlx::query(
            "INSERT INTO paths(path)
            VALUES (?)
            ON CONFLICT(path) DO NOTHING
            RETURNING id",
        )
        .bind(path)
        .fetch_optional(&state.pool)
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg(|| format!("Failed to insert the path {path}!"))?;

        if let Some(row) = inserted_row {
            row.get::<i64, _>("id")
        } else {
            // A concurrent request inserted first.
            sqlx::query("SELECT id FROM paths WHERE path = ? LIMIT 1")
                .bind(path)
                .fetch_one(&state.pool)
                .await
                .ctx(StatusCode::INTERNAL_SERVER_ERROR)
                .log_msg(|| format!("Failed to insert the path {path}!"))?
                .get::<i64, _>("id")
        }
    };

    let visitor_id = state.visitor_states.register(SleepingState {
        path_id,
        registered_at,
    });

    Ok(Json(visitor_id))
}
