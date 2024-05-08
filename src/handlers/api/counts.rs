use axum::{extract::State, Json};
use axum_ctx::*;

use crate::{db::VisitCount, states::AppState};

pub async fn get(State(state): AppState) -> RespResult<Json<Vec<VisitCount>>> {
    VisitCount::all_sorted_by_count(state, None).await.map(Json)
}
