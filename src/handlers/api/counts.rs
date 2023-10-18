use axum::{extract::State, Json};
use oxi_axum_helpers::RespErr;

use crate::{db::VisitCount, states::AppState};

pub async fn get(State(state): AppState) -> Result<Json<Vec<VisitCount>>, RespErr> {
    VisitCount::all_sorted(&state.pool).await.map(Json)
}
