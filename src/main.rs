mod app;
mod config;
mod db;
mod handlers;

use axum::Server;
use oxi_axum_helpers::{runner, shutdown_signal, InitErr, InitErrCtx};
use tracing::info;

use app::app;

async fn init() -> Result<(), InitErr> {
    let (app, socket_address) = app().await?;

    info!("Listening on {socket_address}");
    Server::bind(&socket_address)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .init_ctx("Server error!")
}

fn main() {
    runner::run(init);
}
