use askama::Template;
use askama_web::WebTemplate;
use axum::{extract::State, response::IntoResponse};

use crate::{handlers::base_template::Base, states::AppState};

#[derive(Template, WebTemplate)]
#[template(path = "index.html")]
struct Index<'a> {
    pub base: Base<'a>,
    pub base_url: &'static str,
    pub tracked_origin: &'static str,
}

pub async fn get(State(state): AppState) -> impl IntoResponse {
    Index {
        base: Base::new(state, "Dashboard"),
        base_url: state.base_url,
        tracked_origin: state.tracked_origin,
    }
}
