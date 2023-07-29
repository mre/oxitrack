pub mod sleeping_hotel;

use std::{path::PathBuf, sync::Mutex};

use oxi_axum_helpers::{InitErr, InitErrCtx};
use time::UtcOffset;

use crate::{config::Config, db::Database};
use sleeping_hotel::SleepingHotel;

/// The application state.
pub struct AppState {
    pub db: Database,
    pub file_content: &'static [u8],
    pub mime: String,
    pub tracked_base_url: String,
    pub sleeping_hotel: Mutex<SleepingHotel<i64, 14, 60>>,
    pub utc_offset: UtcOffset,
}

impl AppState {
    pub async fn build(
        data_dir: PathBuf,
        config: Config,
        utc_offset: UtcOffset,
    ) -> Result<Self, InitErr> {
        let db = Database::build(config.db).await?;

        let response_file = data_dir.join(config.response_filename);
        let file_content = tokio::fs::read(&response_file)
            .await
            .init_ctx_lz(|| {
                format!(
                    "Failed to open the response file {}",
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
            sleeping_hotel: Mutex::new(SleepingHotel::default()),
            utc_offset,
        })
    }
}
