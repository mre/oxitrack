use askama::Template;
use axum::{
    extract::{Query, State},
    response::Response,
};
use axum_ctx::{RespErr, RespErrCtx, RespErrExt, Status};
use bigdecimal::ToPrimitive;
use oxi_axum_helpers::TryIntoTemplResp;
use sqlx::PgPool;
use std::{fmt, num::NonZeroU64};
use time::OffsetDateTime;

use crate::{
    extractors::query_path::QueryPath,
    handlers::{
        base_template::Base,
        chart_data::{TotalLen, WholeDaysSinceFirstVisit},
    },
    states::{AppState, InnerAppState},
};

use super::count_rows::{Count, CountRows};

struct Seconds(u64);

impl fmt::Display for Seconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let minutes = self.0 / 60;

        if minutes > 0 {
            write!(f, "{}min {:02}s", self.0 / 60, self.0 % 60)
        } else {
            write!(f, "{:02}s", self.0 % 60)
        }
    }
}

struct DateTimeVerboseFormatter(OffsetDateTime);

impl fmt::Display for DateTimeVerboseFormatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {:02}:{:02} UTC{}",
            self.0.date(),
            self.0.hour(),
            self.0.minute(),
            self.0.offset(),
        )
    }
}

struct Visits {
    first: DateTimeVerboseFormatter,
    len: NonZeroU64,
    per_day: f64,
    average_time_spent: Option<Seconds>,
}

impl Visits {
    async fn build(state: &InnerAppState, path_id: i64) -> Result<Self, RespErr> {
        let WholeDaysSinceFirstVisit {
            first_visit,
            days_since_first_visit,
            ..
        } = WholeDaysSinceFirstVisit::build(&state.pool, path_id, None).await?;

        let average_time_spent = sqlx::query!(
            "SELECT EXTRACT(EPOCH FROM AVG(left_at - registered_at)) FROM visits
            WHERE path_id = $1",
            path_id
        )
        .fetch_one(&state.pool)
        .await
        .ctx(Status::Internal)
        .log_msg("Failed to run the average time spent query!")?
        .extract
        .and_then(|decimal| decimal.to_u64().map(Seconds));

        let len = TotalLen::build(&state.pool, path_id).await?;

        #[allow(clippy::cast_precision_loss)]
        let visits_per_day = if days_since_first_visit > 0 {
            len.inner().get() as f64 / days_since_first_visit as f64
        } else {
            len.inner().get() as f64
        };

        let first_visit = state.apply_utc_offset(first_visit)?;

        Ok(Self {
            first: DateTimeVerboseFormatter(first_visit),
            len: len.inner(),
            per_day: visits_per_day,
            average_time_spent,
        })
    }
}

struct ReferrerCount {
    domain: String,
    count: i64,
}

impl ReferrerCount {
    async fn all_sorted_by_count(pool: &PgPool, path_id: i64) -> Result<Vec<Self>, RespErr> {
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
        .log_msg("Failed to query referrers!")
    }
}

impl Count for ReferrerCount {
    fn count(&self) -> i64 {
        self.count
    }
}

#[derive(Template)]
#[template(path = "stats.html")]
struct Stats<'a> {
    pub base: Base<'a>,
    pub base_url: &'static str,
    pub tracked_origin: &'static str,
    pub path: &'a str,
    pub visits: Visits,
    pub referrer_count_rows: CountRows<ReferrerCount>,
}

pub async fn get(
    State(state): AppState,
    Query(path): Query<QueryPath>,
) -> Result<Response, RespErr> {
    let (path, path_id) = path.normalized_with_id(&state.pool).await?;

    // Run queries concurrently.
    let visits_handler = tokio::spawn(Visits::build(state, path_id));

    let referrer_counts = ReferrerCount::all_sorted_by_count(&state.pool, path_id).await?;
    let referrer_count_rows = CountRows::from(referrer_counts);

    let visits = visits_handler
        .await
        .ctx(Status::Internal)
        .log_msg("Visits task panicked!")??;

    Stats {
        base: Base::new(path),
        base_url: state.base_url,
        tracked_origin: state.tracked_origin,
        path,
        visits,
        referrer_count_rows,
    }
    .try_into_resp()
}
