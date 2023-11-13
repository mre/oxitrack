use askama::Template;
use axum::{
    extract::{Query, State},
    response::Html,
};
use axum_ctx::{RespErr, RespErrCtx, RespErrExt, Status};
use bigdecimal::ToPrimitive;
use oxi_axum_helpers::TryIntoTemplResp;
use sqlx::PgPool;

use crate::{
    extractors::query_path::{PathId, QueryPath},
    formatters::{DateTimeVerboseFormatter, SecondsFormatter},
    handlers::{
        base_template::Base,
        count_rows::{Count, CountRows},
        stats_data::WholeDaysSinceFirstVisit,
    },
    states::{AppState, InnerAppState},
};

struct Visits {
    first: DateTimeVerboseFormatter,
    total_n: i64,
    per_day: f64,
    average_time_spent: Option<SecondsFormatter>,
}

impl Visits {
    async fn build(state: &InnerAppState, path_id: i64) -> Result<Self, RespErr> {
        let WholeDaysSinceFirstVisit {
            whole_days_since_first_visit,
            first_visit,
            ..
        } = WholeDaysSinceFirstVisit::build(&state.pool, Some(path_id), None).await?;

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
        .and_then(|decimal| decimal.to_u64().map(SecondsFormatter));

        #[allow(clippy::cast_sign_loss)]
        let total_n_visits = sqlx::query!(
            r#"SELECT COUNT(*) AS "count!" FROM visits
            WHERE path_id = $1"#,
            path_id,
        )
        .fetch_one(&state.pool)
        .await
        .ctx(Status::Internal)
        .log_msg("Failed to query the count of visits")?
        .count;

        #[allow(clippy::cast_precision_loss)]
        let visits_per_day = if whole_days_since_first_visit > 0 {
            total_n_visits as f64 / whole_days_since_first_visit as f64
        } else {
            total_n_visits as f64
        };

        let first_visit = state.apply_utc_offset(first_visit)?;

        Ok(Self {
            first: DateTimeVerboseFormatter(first_visit),
            total_n: total_n_visits,
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
) -> Result<Html<String>, RespErr> {
    let PathId { path, path_id } = path.normalized_with_id(&state.pool).await?;

    // Run queries concurrently.
    let visits_handler = tokio::spawn(Visits::build(state, path_id));

    let referrer_counts = ReferrerCount::all_sorted_by_count(&state.pool, path_id).await?;
    let referrer_count_rows = CountRows::from(referrer_counts);

    let visits = visits_handler
        .await
        .ctx(Status::Internal)
        .log_msg("Visits task panicked!")??;

    Stats {
        base: Base::new(state, path),
        base_url: state.base_url,
        tracked_origin: state.tracked_origin,
        path,
        visits,
        referrer_count_rows,
    }
    .try_into_resp()
}
