mod config;
mod db;
mod handlers;

use axum::{
    routing::{get, Router},
    Server,
};
use oxi_axum_helpers::{runner, static_handler, InitErr, InitErrCtx, PreTracer};
use rust_embed::RustEmbed;
use std::{net::SocketAddr, sync::Arc};
use tracing::info;

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

    let app_state = Arc::new(AppState::build(config, utc_offset).await?);

    let api_router = Router::new()
        .route("/history", get(handlers::api::history))
        .route("/counts", get(handlers::api::counts));

    let dashboard_router = Router::new()
        .route("/", get(handlers::dashboard::index))
        .route("/plot", get(handlers::dashboard::plot));

    let router = Router::new()
        .route("/static/:file", get(static_handler::handler::<Static>))
        .route("/register", get(handlers::register))
        .route("/post-sleep/:registration_id", get(handlers::post_sleep))
        .nest("/api", api_router)
        .nest("/dashboard", dashboard_router)
        .with_state(app_state);

    info!("Listening on {socket_address}");
    Server::bind(&socket_address)
        .serve(router.into_make_service())
        .await
        .init_ctx("Server error!")
}

fn main() {
    runner::run(init);
}
