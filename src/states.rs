pub mod sleeping_hotel;

use askama::Template;
use axum::extract::State;
use oxi_axum_helpers::{InitErr, InitErrCtx};
use std::sync::Mutex;
use time::UtcOffset;

use crate::{config::Config, db::Database};
use sleeping_hotel::SleepingHotel;

#[derive(Template)]
#[template(path = "count.js", escape = "none")]
pub struct CountJs<'a> {
    pub base_url: &'a str,
    pub sleep_secs: u64,
}

/// The application state.
pub struct InnerAppState {
    pub db: Database,
    pub tracked_origin: String,
    pub tracked_origin_callback: String,
    pub sleeping_hotel: Mutex<SleepingHotel<i64>>,
    pub utc_offset: UtcOffset,
    pub count_js: &'static str,
}

impl InnerAppState {
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

        let count_js = CountJs {
            base_url: &config.base_url,
            sleep_secs: config.min_delay_secs + 1,
        }
        .render()
        .init_ctx("Failed to build the count.js script!")?
        .leak();

        Ok(Self {
            db,
            tracked_origin_callback,
            tracked_origin: config.tracked_origin,
            sleeping_hotel: Mutex::new(sleeping_hotel),
            utc_offset,
            count_js,
        })
    }

    pub fn tracked_url_from_path(&self, path: &str) -> String {
        let mut url = String::with_capacity(self.tracked_origin_callback.len() + path.len());
        url.push_str(&self.tracked_origin_callback);
        url.push_str(path);

        url
    }
}

pub type AppState = State<&'static InnerAppState>;
