use axum::{extract::State, Json};
use axum_ctx::RespErr;

use crate::{db::VisitCount, states::AppState};

pub async fn get(State(state): AppState) -> Result<Json<Vec<VisitCount>>, RespErr> {
    VisitCount::all_sorted_by_count(&state.pool, None)
        .await
        .map(Json)
}
