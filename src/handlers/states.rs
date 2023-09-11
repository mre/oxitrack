pub mod sleeping_hotel;

use oxi_axum_helpers::{InitErr, InitErrCtx};
use std::sync::Mutex;
use time::UtcOffset;

use crate::{config::Config, db::Database};
use sleeping_hotel::SleepingHotel;

/// The application state.
pub struct AppState {
    pub db: Database,
    pub tracked_origin: String,
    pub tracked_origin_callback: String,
    pub sleeping_hotel: Mutex<SleepingHotel<i64>>,
    pub utc_offset: UtcOffset,
}

impl AppState {
    pub async fn build(config: Config, utc_offset: UtcOffset) -> Result<Self, InitErr> {
        let db = Database::build(config.db).await?;

        let tracked_origin_callback = config
            .tracked_origin_callback
            .unwrap_or_else(|| config.tracked_origin.clone());

        let sleeping_hotel = SleepingHotel::new(config.min_delay_secs);

        let callback_connection_error = "Failed to connect to the tracked website using the configuration option tracked_origin_callback/tracked_origin!";
        let callback_status = reqwest::get(&tracked_origin_callback)
            .await
            .init_ctx(callback_connection_error)?
            .status();
        if !callback_status.is_success() {
            return InitErr::new(callback_connection_error);
        }

        Ok(Self {
            db,
            tracked_origin_callback,
            tracked_origin: config.tracked_origin,
            sleeping_hotel: Mutex::new(sleeping_hotel),
            utc_offset,
        })
    }

    pub fn tracked_url_from_path(&self, path: &str) -> String {
        let mut url = String::with_capacity(self.tracked_origin_callback.len() + path.len());
        url.push_str(&self.tracked_origin_callback);
        url.push_str(path);

        url
    }
}
