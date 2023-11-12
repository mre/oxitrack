use anyhow::{Context, Result};
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
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
    pub min_delay_secs: u64,
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
const fn default_min_delay_secs() -> u64 {
    19
}
fn default_logs_dir() -> PathBuf {
    PathBuf::from("/var/log/oxitraffic")
}

impl ConfigBuilder for Config {
    fn build(config_file: &Path) -> Result<Self> {
        Figment::new()
            .merge(
                Env::prefixed("OXITRAFFIC_")
                    .split("__")
                    .ignore(&["config_file"]),
            )
            .join(Toml::file(config_file))
            .extract()
            .context("Failed to parse the configuration!")
    }

    fn hm_utc_offset(&self) -> &HMUtcOffset {
        &self.utc_offset
    }

    fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }
}

#[cfg(test)]
mod tests {
    use super::Config;
    use figment::Jail;
    use oxi_axum_helpers::ConfigBuilder;
    use std::path::Path;

    fn test_config(config_file_content: &str) {
        Jail::expect_with(|jail| {
            jail.set_env("OXITRAFFIC_CONFIG_FILE", "config.toml");

            jail.create_file("config.toml", config_file_content)?;

            Config::build(Path::new("config.toml")).unwrap();

            Ok(())
        });
    }

    #[test]
    fn minimal_config() {
        test_config(
            r#"
            base_url = "https://oxitraffic.mo8it.com"
            tracked_origin = "https://mo8it.com"

            [db]
            host = "127.0.0.1"
            port = 5432
            username = "postgres"
            password = "CHANGE_ME"
            database = "postgres"
            "#,
        )
    }

    #[test]
    fn full_config() {
        test_config(
            r#"
            socket_address = "127.0.0.1:8080"
            base_url = "http://127.0.0.1:8080"
            tracked_origin = "https://mo8it.com"
            tracked_origin_callback = "http://mo8it_com"
            logs_dir = "logs"

            min_delay_secs = 20

            [db]
            host = "127.0.0.1"
            port = 5432
            username = "postgres"
            password = "CHANGE_ME"
            database = "postgres"

            [utc_offset]
            hours = 2
            minutes = 0
            "#,
        )
    }
}
