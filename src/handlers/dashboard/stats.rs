use askama::Template;
use axum::{
    extract::{Query, State},
    response::Response,
};
use futures::TryStreamExt;
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status, TryIntoTemplResp};
use serde::Serialize;
use sqlx::{types::BigDecimal, PgPool};
use time::format_description::well_known::Rfc3339;

use crate::{
    handlers::{base_template::Base, queries::PathQuery},
    states::AppState,
};

#[derive(Serialize)]
struct DataPoint {
    x: String,
    y: i64,
}

struct Visits {
    chart_json_data: String,
    chart_max_count: i64,
    chart_date_trunc: &'static str,
    first: String,
    len: i64,
    per_day: f64,
    average_time_spent_secs: Option<BigDecimal>,
}

impl Visits {
    async fn build(pool: &PgPool, path_id: i64) -> Result<Self, RespErr> {
        let len = sqlx::query!(
            r#"SELECT COUNT(*) AS "count!" FROM visits
            WHERE path_id = $1"#,
            path_id,
        )
        .fetch_one(pool)
        .await
        .ctx(Status::Internal)
        .err_msg("Failed to query the count of visits")?
        .count;

        if len < 1 {
            return Err(RespErr::new(Status::NotFound)
                .user_msg("The requested path has no counted visits yet."));
        }

        let first_visit = sqlx::query!(
            "SELECT timestamp FROM visits
            WHERE path_id = $1
            ORDER BY timestamp
            LIMIT 1",
            path_id,
        )
        .fetch_one(pool)
        .await
        .ctx(Status::Internal)
        .user_msg("Failed to query the first visit")?
        .timestamp;

        let first_visit_formatted = first_visit
            .format(&Rfc3339)
            .ctx(Status::Internal)
            .err_msg("Failed to format the datetime of the first visit!")?;

        let now = time::OffsetDateTime::now_utc();
        let days_since_first_visit = (now - first_visit).whole_days();
        let visits_per_day = if days_since_first_visit > 0 {
            len as f64 / days_since_first_visit as f64
        } else {
            len as f64
        };

        let (chart_data, chart_date_trunc) = if days_since_first_visit > 1460 {
            // More than 4 years.
            let data = sqlx::query!(
                r#"SELECT date_trunc('year', timestamp) AS "trunc_timestamp!",
                COUNT(timestamp) AS "count!" FROM visits
                WHERE path_id = $1
                GROUP BY "trunc_timestamp!"
                ORDER BY "trunc_timestamp!""#,
                path_id,
            )
            .fetch(pool)
            .map_ok(|row| DataPoint {
                x: row.trunc_timestamp.year().to_string(),
                y: row.count,
            })
            .try_collect::<Vec<_>>()
            .await;

            (data, "year")
        } else if days_since_first_visit > 62 {
            // More than 2 months.
            let data = sqlx::query!(
                r#"SELECT date_trunc('month', timestamp) AS "trunc_timestamp!",
                COUNT(timestamp) AS "count!" FROM visits
                WHERE path_id = $1
                GROUP BY "trunc_timestamp!"
                ORDER BY "trunc_timestamp!""#,
                path_id,
            )
            .fetch(pool)
            .map_ok(|row| DataPoint {
                x: row.trunc_timestamp.month().to_string(),
                y: row.count,
            })
            .try_collect::<Vec<_>>()
            .await;

            (data, "month")
        } else {
            let data = sqlx::query!(
                r#"SELECT date_trunc('day', timestamp) AS "trunc_timestamp!",
                COUNT(timestamp) AS "count!" FROM visits
                WHERE path_id = $1
                GROUP BY "trunc_timestamp!"
                ORDER BY "trunc_timestamp!""#,
                path_id,
            )
            .fetch(pool)
            .map_ok(|row| DataPoint {
                x: row.trunc_timestamp.date().to_string(),
                y: row.count,
            })
            .try_collect::<Vec<_>>()
            .await;

            (data, "day")
        };

        let chart_data = chart_data
            .ctx(Status::Internal)
            .err_msg("Failed to query chart data!")?;

        let chart_json_data = serde_json::to_string(&chart_data)
            .ctx(Status::Internal)
            .err_msg("Failed to convert history to JSON string!")?;

        let chart_max_count = chart_data
            .iter()
            .map(|point| point.y)
            .max()
            .unwrap_or_default();

        // TODO: In minutes instead of seconds.
        let average_time_spent_secs = sqlx::query!(
            "SELECT EXTRACT(EPOCH FROM AVG(left_at - timestamp)) FROM visits
            WHERE path_id = $1",
            path_id
        )
        .fetch_one(pool)
        .await
        .ctx(Status::Internal)
        .err_msg("Failed to run the average time spent query!")?
        .extract
        .map(|decimal| decimal.round(0));

        Ok(Self {
            chart_json_data,
            chart_max_count,
            chart_date_trunc,
            first: first_visit_formatted,
            len,
            per_day: visits_per_day,
            average_time_spent_secs,
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

pub async fn get(
    State(state): AppState,
    Query(path): Query<PathQuery>,
) -> Result<Response, RespErr> {
    let path = path.normalized();

    let path_id = sqlx::query!(
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
