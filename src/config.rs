use oxi_axum_helpers::{ConfigBuilder, HMUtcOffset, PgConfig};
use serde::Deserialize;
use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::{Path, PathBuf},
};

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
    #[serde(default = "default_logs_dir")]
    pub logs_dir: PathBuf,
}
const fn default_socket_address() -> SocketAddr {
    // 0.0.0.0:80
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 80))
}
const fn default_min_delay_secs() -> u16 {
    19
}
fn default_logs_dir() -> PathBuf {
    PathBuf::from("/var/log/oxitraffic")
}

impl ConfigBuilder for Config {
    fn hm_utc_offset(&self) -> &HMUtcOffset {
        &self.utc_offset
    }

    fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }
}
