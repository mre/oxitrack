use axum::{extract::State, Json};
use oxi_axum_helpers::RespErr;

use crate::{db::Count, states::AppState};

pub async fn get(State(state): AppState) -> Result<Json<Vec<Count>>, RespErr> {
    Count::query_all_sorted(&state.db).await.map(Json)
}
