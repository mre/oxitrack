pub mod api;
mod base_template;
pub mod dashboard;
pub mod states;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status};
use reqwest::StatusCode;
use serde::Deserialize;
use std::sync::Arc;
use tracing::instrument;

use crate::db::Id;

use self::states::{sleeping_hotel::SleepingHotelInd, AppState};

pub type AppStateT = State<Arc<AppState>>;

#[derive(Deserialize)]
pub struct PathQuery {
    pub path: String,
}

#[instrument(skip_all)]
pub async fn register(
    State(state): AppStateT,
    Query(PathQuery { path }): Query<PathQuery>,
) -> Result<Json<u16>, RespErr> {
    let path = path.trim_end_matches('/');

    let path_id = sqlx::query_as!(Id, "SELECT id FROM paths WHERE path = $1", path)
        .fetch_optional(&*state.db)
        .await
        .ctx(Status::Internal)
        .err_msg("Failed to run path query!")?;

    let path_id = match path_id {
        Some(Id { id }) => id,
        None => {
            let status = reqwest::get(format!("{}/{path}", state.tracked_base_url))
                .await
                .ctx(Status::Internal)
                .err_msg("Failed to look up the path on the tracked website!")?
                .status();

            if status != StatusCode::OK {
                return Err(
                    RespErr::new(Status::NotFound).err_msg("Path not found on tracked website!")
                );
            }

            sqlx::query_as!(Id, "INSERT INTO paths(path) VALUES ($1) RETURNING id", path)
                .fetch_one(&*state.db)
                .await
                .ctx(Status::Internal)
                .err_msg("Failed to insert path!")?
                .id
        }
    };

    let registration_id = state.sleeping_hotel.lock().unwrap().reserve_bed(path_id);

    Ok(Json(registration_id))
}

#[instrument(skip_all)]
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

    sqlx::query!("INSERT INTO calls(path_id) VALUES ($1)", path_id)
        .execute(&*state.db)
        .await
        .ctx(Status::Internal)
        .err_msg("Failed to insert call!")?;

    Ok(StatusCode::OK)
}
