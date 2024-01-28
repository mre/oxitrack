use oxi_axum_helpers::PgConfig;
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
    pub db: PgConfig,
    #[serde(default)]
    pub utc_offset: HMUtcOffset,
}
const fn default_socket_address() -> SocketAddr {
    // 0.0.0.0:80
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 80))
}
const fn default_min_delay_secs() -> u16 {
    19
}
