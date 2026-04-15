use askama::Template;
use askama_web::WebTemplate;
use axum::{
    extract::{Query, State},
    response::Html,
};
use axum_ctx::*;
#[cfg(feature = "postgres")]
use bigdecimal::ToPrimitive;
use sqlx::Row;

use crate::{
    extractors::query_path::{PathId, QueryPath},
    formatters::{DateTimeVerboseFormatter, SecondsFormatter},
    handlers::{base_template::Base, stats_data::WholeDaysSinceFirstVisit},
    states::{AppState, InnerAppState},
};

struct Visits {
    first: DateTimeVerboseFormatter,
    total_n: i64,
    per_day: f64,
    average_time_spent: Option<SecondsFormatter>,
}

impl Visits {
    async fn build(state: &'static InnerAppState, path_id: i64) -> RespResult<Self> {
        let now = state.now_tz()?;

        let Some(WholeDaysSinceFirstVisit {
            whole_days_since_first_visit,
            first_visit,
        }) = WholeDaysSinceFirstVisit::build(state, Some(path_id), now, None).await?
        else {
            return Err(RespErr::new(StatusCode::NOT_FOUND)
                .user_msg("The requested path has no counted visits yet."));
        };

        // -- AVG query -------------------------------------------------------
        #[cfg(feature = "postgres")]
        let sql_avg = "SELECT AVG(time_s) AS avg FROM visits WHERE path_id = $1";
        #[cfg(feature = "sqlite")]
        let sql_avg = "SELECT AVG(time_s) AS avg FROM visits WHERE path_id = ?";

        let avg_row = sqlx::query(sql_avg)
            .bind(path_id)
            .fetch_one(&state.pool)
            .await
            .ctx(StatusCode::INTERNAL_SERVER_ERROR)
            .log_msg("Failed to run the average time spent query!")?;

        #[cfg(feature = "postgres")]
        let average_time_spent = avg_row
            .try_get::<Option<bigdecimal::BigDecimal>, _>("avg")
            .ok()
            .flatten()
            .and_then(|decimal| decimal.to_u64().map(SecondsFormatter));

        #[cfg(feature = "sqlite")]
        let average_time_spent = avg_row
            .try_get::<Option<f64>, _>("avg")
            .ok()
            .flatten()
            .map(|f| SecondsFormatter(f as u64));

        // -- COUNT query -----------------------------------------------------
        #[cfg(feature = "postgres")]
        let sql_count = r#"SELECT COUNT(*) AS count FROM visits WHERE path_id = $1"#;
        #[cfg(feature = "sqlite")]
        let sql_count = "SELECT COUNT(*) AS count FROM visits WHERE path_id = ?";

        #[allow(clippy::cast_sign_loss)]
        let total_n_visits: i64 = sqlx::query(sql_count)
            .bind(path_id)
            .fetch_one(&state.pool)
            .await
            .ctx(StatusCode::INTERNAL_SERVER_ERROR)
            .log_msg("Failed to query the count of visits")?
            .get("count");

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

#[derive(Template, WebTemplate)]
#[template(path = "stats.html")]
struct Stats<'a> {
    pub base: Base<'a>,
    pub base_url: &'static str,
    pub tracked_origin: &'static str,
    pub path: &'a str,
    pub visits: Visits,
}

pub async fn get(
    State(state): AppState,
    Query(path): Query<QueryPath>,
) -> RespResult<Html<String>> {
    let PathId { path, path_id } = path.normalized_with_id(&state.pool).await?;

    let visits = Visits::build(state, path_id).await?;

    Stats {
        base: Base::new(state, path),
        base_url: state.base_url,
        tracked_origin: state.tracked_origin,
        path,
        visits,
    }
    .render()
    .map(Html)
    .ctx(StatusCode::INTERNAL_SERVER_ERROR)
    .log_msg("Failed to render dashboard stats")
}
