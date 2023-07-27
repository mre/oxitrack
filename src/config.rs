use std::path::Path;

use figment::{
    providers::{Env, Format, Yaml},
    Figment,
};
use oxi_axum_helpers::{ConfigBuilder, HMUtcOffset, InitErr, InitErrCtx};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Database {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub database: String,
}

/// Configuration.
#[derive(Deserialize)]
pub struct Config {
    /// The server socket address including port.
    #[serde(default = "default_socket_address")]
    pub socket_address: String,
    pub db: Database,
    #[serde(default)]
    pub utc_offset: HMUtcOffset,
    pub response_filename: String,
    pub tracked_base_url: String,
}
fn default_socket_address() -> String {
    "0.0.0.0:80".to_owned()
}

impl ConfigBuilder for Config {
    fn build(data_dir: &Path) -> Result<Self, InitErr> {
        let config_file_path = data_dir.join("config.yaml");

        Figment::new()
            .merge(Env::prefixed("OXITRAFFIC_").split("__"))
            .join(Yaml::file(config_file_path))
            .extract()
            .init_ctx("Failed to parse the configuration!")
    }

    fn hm_utc_offset(&self) -> &HMUtcOffset {
        &self.utc_offset
    }
}
