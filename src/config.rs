use std::path::Path;

use figment::{
    providers::{Env, Format, Yaml},
    Figment,
};
use oxi_axum_helpers::{ConfigBuilder, DBConfig, HMUtcOffset, InitErr, InitErrCtx};
use serde::Deserialize;

/// Configuration.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The server socket address including port.
    #[serde(default = "default_socket_address")]
    pub socket_address: String,
    pub db: DBConfig,
    #[serde(default)]
    pub utc_offset: HMUtcOffset,
    pub tracked_origin: String,
    #[serde(default)]
    pub tracked_origin_callback: Option<String>,
    #[serde(default = "default_min_delay_secs")]
    pub min_delay_secs: u64,
}
fn default_socket_address() -> String {
    "0.0.0.0:80".to_owned()
}
const fn default_min_delay_secs() -> u64 {
    19
}

impl ConfigBuilder for Config {
    fn build(data_dir: &Path) -> Result<Self, InitErr> {
        let config_file_path = data_dir.join("config.yaml");

        Figment::new()
            .merge(
                Env::prefixed("OXITRAFFIC_")
                    .split("__")
                    .ignore(&["data_dir"]),
            )
            .join(Yaml::file(config_file_path))
            .extract()
            .init_ctx("Failed to parse the configuration!")
    }

    fn hm_utc_offset(&self) -> &HMUtcOffset {
        &self.utc_offset
    }
}
