use axum::{extract::State, Json};
use oxi_axum_helpers::RespErr;

use crate::{db::Count, handlers::AppStateT};

pub async fn get(State(state): AppStateT) -> Result<Json<Vec<Count>>, RespErr> {
    Count::query_all(&state.db).await.map(Json)
}
