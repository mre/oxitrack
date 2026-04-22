use axum::{Json, extract::State};
use axum_ctx::RespResult;

use crate::{db::VisitCount, states::AppState};

pub async fn get(State(state): AppState) -> RespResult<Json<Vec<VisitCount>>> {
    VisitCount::all_sorted_by_count(state, None).await.map(Json)
}
