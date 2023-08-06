use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::Path,
};

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
    pub socket_address: SocketAddr,
    pub db: DBConfig,
    #[serde(default)]
    pub utc_offset: HMUtcOffset,
    pub tracked_origin: String,
    #[serde(default)]
    pub tracked_origin_callback: Option<String>,
    #[serde(default = "default_min_delay_secs")]
    pub min_delay_secs: u64,
}
fn default_socket_address() -> SocketAddr {
    // 0.0.0.0:80
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 80))
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

#[cfg(test)]
mod tests {
    use super::Config;
    use crate::app::DATA_DIR_ENV_VAR;
    use figment::Jail;
    use oxi_axum_helpers::ConfigBuilder;
    use std::path::Path;

    fn test_config(config_file_content: &str) {
        Jail::expect_with(|jail| {
            jail.set_env(DATA_DIR_ENV_VAR, ".");

            jail.create_file("config.yaml", config_file_content)?;

            Config::build(Path::new(".")).unwrap();

            Ok(())
        });
    }

    #[test]
    fn minimal_config() {
        test_config(
            r#"
                tracked_origin: https://mo8it.com

                db:
                  host: 127.0.0.1
                  port: 5432
                  username: postgres
                  password: CHANGE_ME
                  database: postgres
            "#,
        )
    }

    #[test]
    fn full_config() {
        test_config(
            r#"
                socket_address: 0.0.0.0:80

                tracked_origin: https://mo8it.com
                tracked_origin_callback: http://mo8it_com

                min_delay_secs: 20

                db:
                  host: 127.0.0.1
                  port: 5432
                  username: postgres
                  password: CHANGE_ME
                  database: postgres

                utc_offset:
                  hours: 2
                  minutes: 0
            "#,
        )
    }
}
