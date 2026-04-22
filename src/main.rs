mod app;
mod config;
mod db;
mod extractors;
mod formatters;
mod handlers;
mod states;

use anyhow::{Context, Result};
use tokio::{net::TcpListener, runtime::Runtime, signal};
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

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install the Ctrl+C handler for graceful shutdown!");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install the signal handler for graceful shutdown!")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    eprintln!("Signal received. Starting graceful shutdown.");
}

fn main() -> Result<()> {
    Runtime::new()
        .context("Failed to build the Tokio runtime!")?
        .block_on(init())
}
