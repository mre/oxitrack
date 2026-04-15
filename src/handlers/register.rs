use axum::{
    Json,
    extract::{Query, State},
};
use axum_ctx::*;
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

    #[cfg(feature = "postgres")]
    let sql_select = "SELECT id FROM paths WHERE path = $1 LIMIT 1";
    #[cfg(feature = "sqlite")]
    let sql_select = "SELECT id FROM paths WHERE path = ? LIMIT 1";

    let path_row = sqlx::query(sql_select)
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
        #[cfg(feature = "postgres")]
        let sql_insert = "INSERT INTO paths(path)
            VALUES ($1)
            ON CONFLICT ON CONSTRAINT unique_path DO NOTHING
            RETURNING id";
        #[cfg(feature = "sqlite")]
        let sql_insert = "INSERT INTO paths(path)
            VALUES (?)
            ON CONFLICT(path) DO NOTHING
            RETURNING id";

        let inserted_row = sqlx::query(sql_insert)
            .bind(path)
            .fetch_optional(&state.pool)
            .await
            .ctx(StatusCode::INTERNAL_SERVER_ERROR)
            .log_msg(|| format!("Failed to insert the path {path}!"))?;

        if let Some(row) = inserted_row {
            row.get::<i64, _>("id")
        } else {
            // A concurrent request inserted first.
            #[cfg(feature = "postgres")]
            let sql_select_fallback = "SELECT id FROM paths WHERE path = $1 LIMIT 1";
            #[cfg(feature = "sqlite")]
            let sql_select_fallback = "SELECT id FROM paths WHERE path = ? LIMIT 1";

            sqlx::query(sql_select_fallback)
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
