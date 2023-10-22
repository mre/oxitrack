pub mod visitor_state;

use anyhow::{bail, Context, Result};
use askama::Template;
use axum::extract::State;
use sqlx::PgPool;
use std::time::Duration;
use time::UtcOffset;

use crate::config::Config;
use visitor_state::VisitorStateStore;

#[derive(Template)]
#[template(path = "count.js", escape = "none")]
pub struct CountJs {
    pub base_url: &'static str,
    pub sleep_secs: u64,
}

/// The application state.
pub struct InnerAppState {
    pub pool: PgPool,
    pub tracked_origin: &'static str,
    pub tracked_origin_callback: &'static str,
    pub visitor_states: VisitorStateStore,
    pub utc_offset: UtcOffset,
    pub base_url: &'static str,
    pub count_js: &'static str,
    pub http_client: reqwest::Client,
}

impl InnerAppState {
    pub async fn build(config: Config, utc_offset: UtcOffset) -> Result<Self> {
        let pool = config.db.try_into_pool().await?;

        let tracked_origin: &str = config.tracked_origin.leak();

        let tracked_origin_callback = config
            .tracked_origin_callback
            .map_or(tracked_origin, |callback| callback.leak());

        let http_client = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(10))
            .build()
            .context("Failed to build the reqwest client!")?;

        if !cfg!(debug_assertions) {
            // Migrations must be run manually during development.
            // Therefore, only run migrations in release mode.
            sqlx::migrate!()
                .run(&pool)
                .await
                .context("Failed to run migrations!")?;

            // Only check for connection in release mode to speed up the server starting process.
            let callback_status = http_client
                .get(tracked_origin_callback)
                .send()
                .await
                .with_context(|| {
                    format!(
                        "Failed to connect to the tracked website {}!",
                        tracked_origin_callback
                    )
                })?
                .status();
            if !callback_status.is_success() {
                bail!(
                    "The tracked website {} returned the non-successful status code {}!",
                    tracked_origin_callback,
                    callback_status,
                );
            }
        }

        let visitor_states = VisitorStateStore::new(config.min_delay_secs);

        let base_url = config.base_url.leak();

        let count_js = CountJs {
            base_url,
            sleep_secs: config.min_delay_secs + 1,
        }
        .render()
        .context("Failed to build the count.js script!")?
        .leak();

        Ok(Self {
            pool,
            tracked_origin_callback,
            tracked_origin,
            visitor_states,
            utc_offset,
            base_url,
            count_js,
            http_client,
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
