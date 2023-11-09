mod app;
mod config;
mod db;
mod extractors;
mod formatters;
mod handlers;
mod states;

use anyhow::{Context, Result};
use axum::Server;
use oxi_axum_helpers::shutdown_signal;
use tracing::info;

use app::app;

async fn init() -> Result<()> {
    let (app, socket_address) = app().await?;

    info!("Listening on {socket_address}");
    Server::bind(&socket_address)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Server error!")
}

fn main() -> Result<()> {
    tokio::runtime::Runtime::new()
        .context("Failed to build the Tokio runtime!")?
        .block_on(init())
}
