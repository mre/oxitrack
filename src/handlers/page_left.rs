use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use tracing::error;

use crate::states::{
    AppState,
    visitor_state::{self, VisitorId},
};

/// `/page-left/{visitor_id}/{time_on_page_sec}` — always returns 200.
///
/// The browser fires this on `beforeunload` with `keepalive: true`; the user
/// is already gone by the time we process it, so 4xx/5xx responses just
/// pollute client consoles. Real errors are logged.
pub async fn get(
    State(state): AppState,
    Path((visitor_id, time_on_page_sec)): Path<(VisitorId, u16)>,
) -> StatusCode {
    if time_on_page_sec >= state.min_delay_sec
        && let Err(err) = record(state, visitor_id, time_on_page_sec).await
    {
        error!("page-left failed for visitor_id {visitor_id}: {err:#}");
    }
    StatusCode::OK
}

async fn record(
    state: &'static crate::states::InnerAppState,
    visitor_id: VisitorId,
    time_on_page_sec: u16,
) -> anyhow::Result<()> {
    let mut conn = state.pool.acquire().await?;

    let Some(visit_id) = visitor_state::page_left(&mut conn, visitor_id).await? else {
        return Ok(());
    };

    sqlx::query("UPDATE visits SET time_s = ? WHERE id = ?")
        .bind(i32::from(time_on_page_sec))
        .bind(visit_id)
        .execute(&mut *conn)
        .await?;

    Ok(())
}
