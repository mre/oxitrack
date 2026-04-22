use serde::Deserialize;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct HMUtcOffset {
    #[serde(default)]
    pub hours: i8,
    #[serde(default)]
    pub minutes: i8,
}

/// Configuration for a `SQLite` database.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SqliteConfig {
    /// Path to the `SQLite` database file, e.g. `./oxitrack.db`.
    /// Use `":memory:"` for an in-memory database.
    pub path: String,
}

impl SqliteConfig {
    pub async fn try_into_pool(self) -> anyhow::Result<sqlx::SqlitePool> {
        use anyhow::Context;
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        use std::str::FromStr;

        let options = SqliteConnectOptions::from_str(&self.path)
            .context("Failed to parse the SQLite connection string!")?
            .create_if_missing(true);

        SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .context("Failed to connect to the SQLite database!")
    }
}

/// Configuration.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The server socket address including port.
    #[serde(default = "default_socket_address")]
    pub socket_address: SocketAddr,
    pub base_url: String,
    pub tracked_origin: String,
    #[serde(default)]
    pub tracked_origin_callback: Option<String>,
    #[serde(default = "default_min_delay_secs")]
    pub min_delay_secs: u16,
    pub db: SqliteConfig,
    #[serde(default)]
    pub utc_offset: HMUtcOffset,
}

const fn default_socket_address() -> SocketAddr {
    // 0.0.0.0:80
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 80))
}

const fn default_min_delay_secs() -> u16 {
    19
}
