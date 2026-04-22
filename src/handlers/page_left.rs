use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use axum_ctx::{RespErr, RespErrCtx, RespErrExt, RespResult};

use crate::states::{AppState, visitor_state::VisitorId};

pub async fn get(
    State(state): AppState,
    // `u16` caps `time_on_page_sec` at 65 535 s (≈ 18 h), preventing
    // arbitrarily large values from being stored.
    Path((visitor_id, time_on_page_sec)): Path<(VisitorId, u16)>,
) -> RespResult<StatusCode> {
    let visit_id = state
        .visitor_states
        .page_left(visitor_id)
        .ctx(StatusCode::BAD_REQUEST)
        .user_msg("The visitor ID is invalid or has expired!")?;

    if time_on_page_sec < state.min_delay_sec {
        return Err(RespErr::new(StatusCode::BAD_REQUEST)
            .user_msg("The reported time on page is less than the minimum delay!"));
    }

    let time_on_page_sec = i32::from(time_on_page_sec);

    sqlx::query(
        "UPDATE visits
        SET time_s = ?
        WHERE id = ?",
    )
    .bind(time_on_page_sec)
    .bind(visit_id)
    .execute(&state.pool)
    .await
    .ctx(StatusCode::INTERNAL_SERVER_ERROR)
    .log_msg(|| format!("Failed to set time_s for visit_id {visit_id}"))?;

    Ok(StatusCode::OK)
}
