mod config;
mod db;
mod handlers;

use axum::{
    http::HeaderValue,
    routing::{get, Router},
    Server,
};
use oxi_axum_helpers::{runner, shutdown_signal, static_handler, InitErr, InitErrCtx, PreTracer};
use rust_embed::RustEmbed;
use std::{net::SocketAddr, sync::Arc};
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::{DefaultMakeSpan, DefaultOnRequest, TraceLayer},
};
use tracing::{info, Level};

use handlers::states::AppState;

use crate::config::Config;

const DATA_DIR_ENV_VAR: &str = "OXITRAFFIC_DATA_DIR";

#[derive(RustEmbed)]
#[folder = "static/"]
struct Static;

async fn init() -> Result<(), InitErr> {
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

    let router = Router::new()
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

    info!("Listening on {socket_address}");
    Server::bind(&socket_address)
        .serve(router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .init_ctx("Server error!")
}

fn main() {
    runner::run(init);
}

#[cfg(test)]
mod tests {
    use super::Static;

    #[test]
    fn test_static_files() {
        Static::get("main.css").unwrap();
    }
}
