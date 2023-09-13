use axum::{extract::State, response::Response};
use oxi_axum_helpers::{RespErr, TryIntoTemplResp};

use crate::{
    db::Count,
    handlers::{base_template::Base, AppStateT},
};

use super::templates::Index;

pub async fn get(State(state): AppStateT) -> Result<Response, RespErr> {
    let counts = Count::query_all_sorted(&state.db).await?;

    Index {
        base: Base::new("Dashboard"),
        tracked_origin: &state.tracked_origin,
        counts,
    }
    .try_into_resp()
}
