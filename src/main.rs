mod config;
mod db;
mod handlers;

use axum::{
    routing::{get, Router},
    Server,
};
use axum_extra::routing::RouterExt;
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
        data_dir,
        config,
        utc_offset,
    } = PreTracer::<Config>::init(DATA_DIR_ENV_VAR);

    let socket_address = config
        .socket_address
        .parse::<SocketAddr>()
        .init_ctx("Failed to parse the socket address!")?;

    let app_state = Arc::new(AppState::build(data_dir, config, utc_offset).await?);

    let api_router = Router::new()
        .route_with_tsr("/history", get(handlers::api::history_index))
        .route("/history/*path", get(handlers::api::history))
        .route("/counts", get(handlers::api::counts));

    let dashboard_router = Router::new()
        .route("/", get(handlers::dashboard::index))
        .route_with_tsr("/plot", get(handlers::dashboard::plot_index))
        .route("/plot/*path", get(handlers::dashboard::plot));

    let router = Router::new()
        .route("/static/:file", get(static_handler::handler::<Static>))
        .route_with_tsr("/call", get(handlers::call_index))
        .route("/call/*path", get(handlers::call))
        .nest("/api", api_router)
        .nest("/dashboard", dashboard_router)
        .with_state(app_state);

    info!("Listening on {socket_address}");
    Server::bind(&socket_address)
        .serve(router.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .init_ctx("Server error!")
}

fn main() {
    runner::run(init);
}
