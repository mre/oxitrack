mod app;
mod config;
mod db;
mod extractors;
mod formatters;
mod handlers;
mod states;

use anyhow::{Context, Result};
use oxi_axum_helpers::shutdown_signal;
use tokio::{net::TcpListener, runtime::Runtime};
use tracing::info;

async fn init() -> Result<()> {
    let (app, socket_address) = app::app().await?;

    let listener = TcpListener::bind(socket_address).await?;
    info!(
        "Listening on http://{}",
        listener
            .local_addr()
            .context("Failed to determine the local address to bind to!")?
    );
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("Server error!")
}

fn main() -> Result<()> {
    Runtime::new()
        .context("Failed to build the Tokio runtime!")?
        .block_on(init())
}
