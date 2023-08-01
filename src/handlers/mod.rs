pub mod api;
mod base_template;
pub mod dashboard;
mod queries;
pub mod states;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status};
use reqwest::StatusCode;
use std::sync::Arc;

use crate::db::Id;

use self::{
    queries::PathQuery,
    states::{sleeping_hotel::SleepingHotelInd, AppState},
};

pub type AppStateT = State<Arc<AppState>>;

pub async fn register(
    State(state): AppStateT,
    Query(path): Query<PathQuery>,
) -> Result<Json<u16>, RespErr> {
    let path = path.normalized();

    let path_id = sqlx::query_as!(Id, "SELECT id FROM paths WHERE path = $1", path)
        .fetch_optional(&*state.db)
        .await
        .ctx(Status::Internal)
        .err_msg_lz(|| format!("Failed to run path query for path {path}!"))?;

    let path_id = match path_id {
        Some(Id { id }) => id,
        None => {
            let status = reqwest::get(state.tracked_url_from_path(path))
                .await
                .ctx(Status::NotFound)
                .err_msg_lz(|| {
                    format!("Failed to look up the path {path} on the tracked website!")
                })?
                .status();

            if !status.is_success() {
                return Err(RespErr::new(Status::NotFound)
                    .err_msg(format!("Path {path} not found on tracked website!")));
            }

            // There is a possible race condition here.
            // If two requests to the same new path try to insert it at the same time,
            // then only one insertion will be succussful.
            // If the insertion fails because of the constraint, we will try to select.
            let inserted_id = sqlx::query_as!(
                Id,
                "INSERT INTO paths(path) VALUES ($1)
                ON CONFLICT ON CONSTRAINT unique_path DO NOTHING
                RETURNING id",
                path
            )
            .fetch_optional(&*state.db)
            .await
            .ctx(Status::Internal)
            .err_msg_lz(|| format!("Failed to insert path {path}!"))?;

            match inserted_id {
                Some(id) => id.id,
                None => {
                    // Other request did insert the path first.
                    sqlx::query_as!(
                        Id,
                        "SELECT id FROM paths
                        WHERE path = $1",
                        path
                    )
                    .fetch_one(&*state.db)
                    .await
                    .ctx(Status::Internal)
                    .err_msg_lz(|| format!("Failed to insert path {path}!"))?
                    .id
                }
            }
        }
    };

    let registration_id = state.sleeping_hotel.lock().unwrap().reserve_bed(path_id);

    Ok(Json(registration_id))
}

pub async fn post_sleep(
    State(state): AppStateT,
    Path(registration_id): Path<SleepingHotelInd>,
) -> Result<StatusCode, RespErr> {
    let path_id = state
        .sleeping_hotel
        .lock()
        .unwrap()
        .wake_up(registration_id)
        .ctx(Status::BadRequest)
        .user_msg("The registered ID is invalid or has expired!")?;

    sqlx::query!("INSERT INTO visits(path_id) VALUES ($1)", path_id)
        .execute(&*state.db)
        .await
        .ctx(Status::Internal)
        .err_msg_lz(|| format!("Failed to insert call for path_id {path_id}!"))?;

    Ok(StatusCode::OK)
}
