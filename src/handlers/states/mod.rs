pub mod sleeping_hotel;

use std::sync::Mutex;

use oxi_axum_helpers::InitErr;
use time::UtcOffset;

use crate::{config::Config, db::Database};
use sleeping_hotel::SleepingHotel;

/// The application state.
pub struct AppState {
    pub db: Database,
    pub tracked_origin: String,
    pub sleeping_hotel: Mutex<SleepingHotel<i64, 14, 60>>,
    pub utc_offset: UtcOffset,
}

impl AppState {
    pub async fn build(config: Config, utc_offset: UtcOffset) -> Result<Self, InitErr> {
        let db = Database::build(config.db).await?;

        Ok(Self {
            db,
            tracked_origin: config.tracked_origin,
            sleeping_hotel: Mutex::new(SleepingHotel::default()),
            utc_offset,
        })
    }
}
