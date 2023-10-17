pub mod visitor_state;

use anyhow::{bail, Context, Result};
use askama::Template;
use axum::extract::State;
use time::UtcOffset;

use crate::{config::Config, db::Database};
use visitor_state::VisitorStateStore;

#[derive(Template)]
#[template(path = "count.js", escape = "none")]
pub struct CountJs {
    pub base_url: &'static str,
    pub sleep_secs: u64,
}

/// The application state.
pub struct InnerAppState {
    pub db: Database,
    pub tracked_origin: &'static str,
    pub tracked_origin_callback: &'static str,
    pub visitor_states: VisitorStateStore,
    pub utc_offset: UtcOffset,
    pub base_url: &'static str,
    pub count_js: &'static str,
}

impl InnerAppState {
    pub async fn build(config: Config, utc_offset: UtcOffset) -> Result<Self> {
        let db = Database::build(config.db).await?;

        let tracked_origin: &str = config.tracked_origin.leak();

        let tracked_origin_callback = config
            .tracked_origin_callback
            .map_or(tracked_origin, |callback| callback.leak());

        let visitor_states = VisitorStateStore::new(config.min_delay_secs);

        let callback_connection_error = "Failed to connect to the tracked website using the configuration option tracked_origin_callback/tracked_origin!";
        let callback_status = reqwest::get(tracked_origin_callback)
            .await
            .context(callback_connection_error)?
            .status();
        if !callback_status.is_success() {
            bail!(callback_connection_error);
        }

        let base_url = config.base_url.leak();

        let count_js = CountJs {
            base_url,
            sleep_secs: config.min_delay_secs + 1,
        }
        .render()
        .context("Failed to build the count.js script!")?
        .leak();

        Ok(Self {
            db,
            tracked_origin_callback,
            tracked_origin,
            visitor_states,
            utc_offset,
            base_url,
            count_js,
        })
    }

    pub fn tracked_url_from_path(&self, path: &str) -> String {
        let mut url = String::with_capacity(self.tracked_origin_callback.len() + path.len());
        url.push_str(self.tracked_origin_callback);
        url.push_str(path);

        url
    }
}

pub type AppState = State<&'static InnerAppState>;
