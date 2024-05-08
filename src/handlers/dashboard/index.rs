use askama::Template;
use axum::{extract::State, response::Html};
use axum_ctx::*;
use oxi_axum_helpers::TryIntoTemplResp;

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
    .try_into_resp()
}
