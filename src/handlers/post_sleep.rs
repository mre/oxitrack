use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status};

use super::{states::sleeping_hotel::SleepingHotelInd, AppStateT};

pub async fn get(
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
