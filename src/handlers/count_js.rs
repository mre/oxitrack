use axum::{
    body::Full,
    extract::State,
    http::header::{self, HeaderValue},
    response::{IntoResponse, Response},
};

use super::AppStateT;

static CONTENT_TYPE: HeaderValue = HeaderValue::from_static("text/javascript");
static CACHE_CONTROL: HeaderValue =
    HeaderValue::from_static("public, max-age=86400, must-revalidate");

pub async fn get(State(state): AppStateT) -> Response {
    let mut res = Full::from(state.count_js).into_response();

    res.headers_mut().extend([
        (header::CONTENT_TYPE, CONTENT_TYPE.clone()),
        (header::CACHE_CONTROL, CACHE_CONTROL.clone()),
    ]);

    res
}
