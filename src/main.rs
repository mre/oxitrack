mod config;
mod db;
mod handlers;

use axum::{
    routing::{get, Router},
    Server,
};
use config_builder::ConfigBuilder;
use init_err::{InitErr, InitErrCtx};
use std::{env, net::SocketAddr, path::PathBuf, process, sync::Arc};
use tracing::info;

use config::Config;
use handlers::states::AppState;

const DATA_DIR_ENV_VAR: &str = "OXITRAFFIC_DATA_DIR";

async fn init() -> Result<(), InitErr> {
    let Ok(env_var) = env::var(DATA_DIR_ENV_VAR) else {
        eprintln!("Environment variable {DATA_DIR_ENV_VAR} missing!");
        process::exit(1);
    };
    let data_dir = PathBuf::from(env_var);

    let config = Config::handled_build(&data_dir);

    let socket_address = config
        .socket_address
        .parse::<SocketAddr>()
        .init_ctx("Failed to parse the socket address!")?;

    let app_state = Arc::new(AppState::build(config, data_dir).await?);

    let api_router = Router::new()
        .route("/history/*path", get(handlers::api::history))
        .route("/counts", get(handlers::api::counts));

    let router = Router::new()
        .route("/call/*path", get(handlers::call))
        .nest("/api", api_router)
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
