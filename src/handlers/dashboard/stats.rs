use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Query, State};
use axum_ctx::*;
use serde::Deserialize;
use sqlx::Row;

use crate::{
    extractors::query_path::{PathId, QueryPath},
    formatters::{DateTimeVerboseFormatter, SecondsFormatter},
    handlers::{
        base_template::Base,
        count_rows::CountRows,
        stats_data::{
            Filter, WholeDaysSinceFirstVisit, build_chart, chart_width,
            referrer_count::ReferrerCount, start_datetime_for_filter,
        },
    },
    states::{AppState, InnerAppState},
};

#[derive(Deserialize, Default)]
pub struct FilterQuery {
    #[serde(default)]
    filter: Filter,
}

pub struct Visits {
    pub first: DateTimeVerboseFormatter,
    pub total_n: i64,
    pub per_day: f64,
    pub average_time_spent: Option<SecondsFormatter>,
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

        let avg_row = sqlx::query("SELECT AVG(time_s) AS avg FROM visits WHERE path_id = ?")
            .bind(path_id)
            .fetch_one(&state.pool)
            .await
            .ctx(StatusCode::INTERNAL_SERVER_ERROR)
            .log_msg("Failed to run the average time spent query!")?;

        let average_time_spent = avg_row
            .try_get::<Option<f64>, _>("avg")
            .ok()
            .flatten()
            .map(|f| SecondsFormatter(f as u64));

        let total_n_visits: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM visits WHERE path_id = ?")
                .bind(path_id)
                .fetch_one(&state.pool)
                .await
                .ctx(StatusCode::INTERNAL_SERVER_ERROR)
                .log_msg("Failed to query the count of visits")?;

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
pub struct Stats {
    pub base: Base<'static>,
    pub base_url: &'static str,
    pub tracked_origin: &'static str,
    pub path: String,
    pub visits: Visits,
    pub referrers: CountRows<ReferrerCount>,
    pub chart: Vec<crate::handlers::stats_data::ChartBar>,
    pub chart_width: f64,
    pub filter: Filter,
}

pub async fn get(
    State(state): AppState,
    Query(path_q): Query<QueryPath>,
    Query(filter_q): Query<FilterQuery>,
) -> RespResult<Stats> {
    let filter = filter_q.filter;
    let now = state.now_tz()?;
    let start_datetime = start_datetime_for_filter(filter, now)?;

    let PathId { path, path_id } = path_q.normalized_with_id(&state.pool).await?;

    let visits = Visits::build(state, path_id).await?;

    let referrers =
        ReferrerCount::all_sorted_by_count(state, Some(path_id), start_datetime).await?;
    let referrers = CountRows::from(referrers);

    let chart = build_chart(state, Some(path_id), filter).await?;
    let cw = chart_width(chart.len());

    Ok(Stats {
        base: Base::new(state, "Stats"),
        base_url: state.base_url,
        tracked_origin: state.tracked_origin,
        path: path.to_owned(),
        visits,
        referrers,
        chart,
        chart_width: cw,
        filter,
    })
}
