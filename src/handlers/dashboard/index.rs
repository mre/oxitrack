use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use rinja_axum::Template;

use crate::{handlers::base_template::Base, states::AppState};

#[derive(Template)]
#[template(path = "index.html")]
struct Index<'a> {
    pub base: Base<'a>,
    pub base_url: &'static str,
    pub tracked_origin: &'static str,
}

pub async fn get(State(state): AppState) -> Response {
    Index {
        base: Base::new(state, "Dashboard"),
        base_url: state.base_url,
        tracked_origin: state.tracked_origin,
    }
    .into_response()
}
