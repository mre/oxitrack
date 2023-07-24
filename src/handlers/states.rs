use std::path::PathBuf;

use init_err::{InitErr, InitErrCtx};

use crate::{config::Config, db::Database};

/// The application state.
pub struct AppState {
    pub db: Database,
    pub file_content: &'static [u8],
    pub mime: String,
    pub tracked_base_url: String,
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

        let mime = mime_guess::from_path(response_file).first_or_octet_stream();
        let mime = mime.as_ref().to_string();

        Ok(Self {
            db,
            file_content,
            mime,
            tracked_base_url: config.tracked_base_url,
        })
    }
}
