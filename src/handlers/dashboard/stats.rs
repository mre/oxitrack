use askama::Template;
use axum::{
    extract::{Query, State},
    response::Response,
};
use futures::TryStreamExt;
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status, TryIntoTemplResp};
use sqlx::PgPool;

use crate::{
    db::{Id, TimeStamp},
    handlers::{base_template::Base, queries::PathQuery},
    states::AppState,
};

struct Visits {
    timestamps_json: String,
    len: usize,
    min_chart_timestamp: i64,
    max_chart_timestamp: i64,
    per_day: f64,
}

impl Visits {
    async fn build(pool: &PgPool, path_id: i64) -> Result<Self, RespErr> {
        // Converted to ms timestamp.
        let timestamps = sqlx::query_as!(
            TimeStamp,
            "SELECT timestamp FROM visits
            WHERE path_id = $1
            ORDER BY timestamp",
            path_id,
        )
        .fetch(pool)
        .map_ok(|row| 1000 * row.timestamp.unix_timestamp())
        .try_collect::<Vec<_>>()
        .await
        .ctx(Status::Internal)
        .err_msg("History query failed!")?;

        let len = timestamps.len();

        let min_timestamp = *timestamps
            .first()
            .ctx(Status::NotFound)
            .user_msg("The requested path has no counted visits yet.")?;

        let now_ms = 1000 * time::OffsetDateTime::now_utc().unix_timestamp();
        let ms_since_first_visit = now_ms - min_timestamp;
        let visits_per_day = if ms_since_first_visit > MS_PER_DAY {
            len as f64 * MS_PER_DAY_F / ms_since_first_visit as f64
        } else {
            len as f64
        };

        let x_axis_padding = (ms_since_first_visit as f64 * 0.004) as i64;
        let min_chart_timestamp = min_timestamp - x_axis_padding;
        let max_chart_timestamp = now_ms + x_axis_padding;

        let timestamps_json = serde_json::to_string(&timestamps)
            .ctx(Status::Internal)
            .err_msg("Failed to convert history to JSON string!")?;

        Ok(Self {
            timestamps_json,
            len,
            min_chart_timestamp,
            max_chart_timestamp,
            per_day: visits_per_day,
        })
    }
}

struct Referrer {
    domain: String,
    count: i64,
}

impl Referrer {
    async fn all(pool: &PgPool, path_id: i64) -> Result<Vec<Self>, RespErr> {
        sqlx::query_as!(
            Self,
            r#"SELECT domain, COUNT(*) as "count!" FROM visits
            INNER JOIN referrers ON referrers.id = referrer_id
            WHERE path_id = $1
            GROUP BY domain
            ORDER BY "count!" DESC"#,
            path_id
        )
        .fetch_all(pool)
        .await
        .ctx(Status::Internal)
        .err_msg("Failed to query referrers!")
    }
}

#[derive(Template)]
#[template(path = "stats.html")]
struct Stats<'a> {
    pub base: Base<'a>,
    pub tracked_origin: &'a str,
    pub path: &'a str,
    pub visits: Visits,
    pub referrers: Vec<Referrer>,
}

// Milliseconds per day.
const MS_PER_DAY: i64 = 86_400_000;
const MS_PER_DAY_F: f64 = MS_PER_DAY as f64;

pub async fn get(
    State(state): AppState,
    Query(path): Query<PathQuery>,
) -> Result<Response, RespErr> {
    let path = path.normalized();

    let path_id = sqlx::query_as!(
        Id,
        "SELECT id FROM paths
        WHERE path = $1",
        path
    )
    .fetch_one(&*state.db)
    .await
    .ctx(Status::NotFound)
    .err_msg(|| format!("Path {path} not found!"))?
    .id;

    // Run queries concurrently.
    let visits_handler = tokio::spawn(Visits::build(&state.db, path_id));

    let referrers = Referrer::all(&state.db, path_id).await?;
    let visits = visits_handler
        .await
        .ctx(Status::Internal)
        .err_msg("Visits task panicked!")??;

    Stats {
        base: Base::new(path),
        tracked_origin: &state.tracked_origin,
        path,
        visits,
        referrers,
    }
    .try_into_resp()
}
