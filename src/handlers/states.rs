use std::{collections::HashSet, net::IpAddr, path::PathBuf, sync::Mutex};

use init_err::{InitErr, InitErrCtx};

use crate::{config::Config, db::Database};

/// The application state.
pub struct AppState {
    pub db: Database,
    pub file_content: &'static [u8],
    pub mime: String,
    pub tracked_base_url: String,
    pub anti_spam: Mutex<HashSet<(i64, IpAddr)>>,
}

impl AppState {
    pub async fn build(config: Config, data_dir: PathBuf) -> Result<Self, InitErr> {
        let db = Database::build(config.db).await?;

        let response_file = data_dir.join(config.response_filename);
        let file_content = tokio::fs::read(&response_file)
            .await
            .init_ctx_lz(|| {
                format!(
                    "Failed to open the reponse file {}",
                    response_file.display()
                )
            })?
            .leak();

        let mime = mime_guess::from_path(response_file)
            .first_or_octet_stream()
            .as_ref()
            .to_owned();

        Ok(Self {
            db,
            file_content,
            mime,
            tracked_base_url: config.tracked_base_url,
            anti_spam: Mutex::new(HashSet::with_capacity(1024)),
        })
    }
}
