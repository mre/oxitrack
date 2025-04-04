use askama::Template;
use askama_web::WebTemplate;
use axum::extract::State;

use crate::{handlers::base_template::Base, states::AppState};

#[derive(Template, WebTemplate)]
#[template(path = "index.html")]
pub struct Index {
    pub base: Base<'static>,
    pub base_url: &'static str,
    pub tracked_origin: &'static str,
}

pub async fn get(State(state): AppState) -> Index {
    Index {
        base: Base::new(state, "Dashboard"),
        base_url: state.base_url,
        tracked_origin: state.tracked_origin,
    }
}
