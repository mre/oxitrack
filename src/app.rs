use std::{net::SocketAddr, sync::Arc};

use axum::{http::HeaderValue, routing::get, Router};
use oxi_axum_helpers::{static_handler, InitErr, InitErrCtx, PreTracer};
use rust_embed::RustEmbed;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::{DefaultMakeSpan, DefaultOnRequest, TraceLayer},
};
use tracing::Level;

use crate::{
    config::Config,
    handlers::{self, states::AppState},
};

pub const DATA_DIR_ENV_VAR: &str = "OXITRAFFIC_DATA_DIR";

#[derive(RustEmbed)]
#[folder = "static/"]
struct Static;

pub async fn app() -> Result<(Router, SocketAddr), InitErr> {
    let PreTracer {
        config, utc_offset, ..
    } = PreTracer::<Config>::init(DATA_DIR_ENV_VAR);

    let socket_address = config
        .socket_address
        .parse::<SocketAddr>()
        .init_ctx("Failed to parse the socket address!")?;

    let allowed_origin = config
        .tracked_origin
        .parse::<HeaderValue>()
        .init_ctx("Failed to parse the tracked origin!")?;

    let app_state = Arc::new(AppState::build(config, utc_offset).await?);

    let compression_layer = CompressionLayer::new().gzip(true);

    let counting_router = Router::new()
        .route("/register", get(handlers::register))
        .route("/post-sleep/:registration_id", get(handlers::post_sleep))
        .layer(CorsLayer::new().allow_origin(allowed_origin));

    let api_router = Router::new()
        .route("/history", get(handlers::api::history))
        .route("/counts", get(handlers::api::counts))
        .layer(compression_layer.clone());

    let dashboard_router = Router::new()
        .route("/", get(handlers::dashboard::index))
        .route("/stats", get(handlers::dashboard::stats))
        .layer(compression_layer);

    let app = Router::new()
        .route("/static/:file", get(static_handler::handler::<Static>))
        .merge(counting_router)
        .nest("/api", api_router)
        .merge(dashboard_router)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_request(DefaultOnRequest::new().level(Level::INFO))
                .on_response(())
                .on_body_chunk(())
                .on_eos(())
                .on_failure(()),
        )
        .with_state(app_state);

    Ok((app, socket_address))
}

#[cfg(test)]
mod tests {
    use super::Static;

    #[test]
    fn static_files() {
        Static::get("main.css").unwrap();
    }
}
