pub mod visitor_state;

use anyhow::{bail, Context, Result};
use axum::extract::State;
use axum_ctx::*;
use rinja::Template;
use sqlx::PgPool;
use std::time::Duration;
use time::{OffsetDateTime, UtcOffset};
use url::Url;

use crate::config::Config;
use visitor_state::VisitorStateStore;

#[derive(Template)]
#[template(path = "count.js", escape = "none")]
pub struct CountJs {
    pub base_url: &'static str,
    pub min_delay_secs: u16,
}

/// The application state.
pub struct InnerAppState {
    pub pool: PgPool,
    pub tracked_origin: &'static str,
    pub tracked_origin_callback: &'static str,
    pub visitor_states: VisitorStateStore,
    pub min_delay_sec: u16,
    pub utc_offset: UtcOffset,
    pub utc_offset_str: &'static str,
    pub posix_utc_offset_str: &'static str,
    pub base_url: &'static str,
    pub base_origin: &'static str,
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
                .set_ignore_missing(true)
                .run(&pool)
                .await
                .context("Failed to run migrations!")?;

            // Only check for connection in release mode to speed up the server starting process.
            let callback_status = http_client
                .get(tracked_origin_callback)
                .send()
                .await
                .with_context(|| {
                    format!("Failed to connect to the tracked website {tracked_origin_callback}!")
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
        let base_origin = Url::parse(base_url)
            .context("Failed to parse the base URL configuration value!")?
            .origin();
        if !base_origin.is_tuple() {
            bail!("Failed to parse the origin of the base URL configuration value!");
        }
        let base_origin = base_origin.ascii_serialization().leak();

        let count_js = CountJs {
            base_url,
            min_delay_secs: config.min_delay_secs + 1,
        }
        .render()
        .context("Failed to build the count.js script!")?
        .leak();

        let (utc_offset_h, utc_offset_m, _) = utc_offset.as_hms();
        let utc_offset_str = {
            let sign = if utc_offset.is_negative() { '-' } else { '+' };
            format!("{sign}{utc_offset_h:02}:{utc_offset_m:02}").leak()
        };
        let posix_utc_offset_str = {
            // The opposite sign is important since this is POSIX!
            let posix_sign = if utc_offset.is_negative() { '+' } else { '-' };
            format!("{posix_sign}{utc_offset_h:02}:{utc_offset_m:02}").leak()
        };

        Ok(Self {
            pool,
            tracked_origin,
            tracked_origin_callback,
            visitor_states,
            min_delay_sec: config.min_delay_secs,
            utc_offset,
            utc_offset_str,
            posix_utc_offset_str,
            base_url,
            base_origin,
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

    pub fn apply_utc_offset(&self, datetime: OffsetDateTime) -> RespResult<OffsetDateTime> {
        match datetime.checked_to_offset(self.utc_offset) {
            Some(t) => Ok(t),
            None => Err(RespErr::new(StatusCode::INTERNAL_SERVER_ERROR)
                .log_msg("Failed to change the UTC offset of a datetime!")),
        }
    }

    pub fn now_tz(&self) -> RespResult<OffsetDateTime> {
        self.apply_utc_offset(OffsetDateTime::now_utc())
    }
}

pub type AppState = State<&'static InnerAppState>;
