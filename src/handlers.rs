pub mod api;
mod base_template;
pub mod dashboard;
pub mod post_sleep;
mod queries;
pub mod register;
pub mod states;

use axum::extract::State;

use states::AppState;

pub type AppStateT = State<&'static AppState>;
