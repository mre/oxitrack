use axum::{extract::State, response::Html};
use axum_ctx::{RespErrCtx, RespErrExt, RespResult, StatusCode};
use rinja::Template;

use crate::{handlers::base_template::Base, states::AppState};

#[derive(Template)]
#[template(path = "index.html")]
struct Index<'a> {
    pub base: Base<'a>,
    pub base_url: &'static str,
    pub tracked_origin: &'static str,
}

pub async fn get(State(state): AppState) -> RespResult<Html<String>> {
    Index {
        base: Base::new(state, "Dashboard"),
        base_url: state.base_url,
        tracked_origin: state.tracked_origin,
    }
    .render()
    .map(Html)
    .ctx(StatusCode::INTERNAL_SERVER_ERROR)
    .log_msg("Failed to render dashboard index")
}
