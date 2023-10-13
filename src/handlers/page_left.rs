use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status};

use crate::states::{visitor_state::VisitorId, AppState};

pub async fn get(
    State(state): AppState,
    Path(visitor_id): Path<VisitorId>,
) -> Result<StatusCode, RespErr> {
    let visit_id = state
        .visitor_states
        .page_left(visitor_id)
        .ctx(oxi_axum_helpers::Status::BadRequest)
        .user_msg("The visitor ID is invalid or has expired!")?;

    sqlx::query!(
        "UPDATE visits
        SET left_at = CURRENT_TIMESTAMP(0)
        WHERE id = $1",
        visit_id
    )
    .execute(&*state.db)
    .await
    .ctx(Status::Internal)
    .err_msg(|| format!("Failed to set left_at for visit_id {visit_id}"))?;

    Ok(StatusCode::OK)
}
