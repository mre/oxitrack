mod templates;

use axum::{
    extract::{Path, State},
    response::Response,
};
use resp_err::RespErr;
use tracing::instrument;

use super::AppStateT;

#[instrument(skip_all)]
pub async fn index(State(state): AppStateT) -> Result<Response, RespErr> {
    todo!()
}

#[instrument(skip_all)]
pub async fn plot_index(State(state): AppStateT) -> Result<Response, RespErr> {
    todo!()
}

#[instrument(skip_all)]
pub async fn plot(State(state): AppStateT, Path(path): Path<String>) -> Result<Response, RespErr> {
    todo!()
}
