use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Query, State};
use axum_ctx::{RespErr, RespErrCtx, RespErrExt, RespResult, StatusCode};
use serde::Deserialize;
use sqlx::Row;

use crate::{
    extractors::query_path::{PathId, QueryPath},
    formatters::{DateTimeVerboseFormatter, SecondsFormatter},
    handlers::{
        base_template::Base,
        count_rows::CountRows,
        stats_data::{
            DateRange, WholeDaysSinceFirstVisit, build_chart, referrer_count::ReferrerCount,
        },
    },
    states::{AppState, InnerAppState},
};

#[derive(Deserialize, Default)]
pub struct RangeQuery {
    pub from: Option<String>,
    pub to: Option<String>,
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
        }) = WholeDaysSinceFirstVisit::build(state, Some(path_id), now).await?
        else {
            return Err(RespErr::new(StatusCode::NOT_FOUND)
                .user_msg("The requested path has no counted visits yet."));
        };

        let stats_row = sqlx::query(
            "SELECT COUNT(*) AS total_n, AVG(time_s) AS avg FROM visits WHERE path_id = ?",
        )
        .bind(path_id)
        .fetch_one(&state.pool)
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Failed to query visit stats!")?;

        let total_n_visits: i64 = stats_row.get("total_n");
        let average_time_spent_raw = stats_row.try_get::<Option<f64>, _>("avg").ok().flatten();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let average_time_spent = average_time_spent_raw.map(|f| SecondsFormatter(f as u64));

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
    pub tracked_origin: &'static str,
    pub path: String,
    pub visits: Visits,
    pub referrers: CountRows<ReferrerCount>,
    pub chart: Vec<crate::handlers::stats_data::ChartBar>,
    pub range: DateRange,
}

pub async fn get(
    State(state): AppState,
    Query(path_q): Query<QueryPath>,
    Query(range_q): Query<RangeQuery>,
) -> RespResult<Stats> {
    let now = state.now_tz()?;
    let range = if range_q.from.is_none() && range_q.to.is_none() {
        let fmt = time::macros::format_description!("[year]-[month]-[day]");
        let from = (now.date() - time::Duration::days(90))
            .format(fmt)
            .unwrap_or_default();
        let to = now.date().format(fmt).unwrap_or_default();
        DateRange::from_params(Some(from), Some(to))
    } else {
        DateRange::from_params(range_q.from, range_q.to)
    };

    let PathId { path, path_id } = path_q.normalized_with_id(&state.pool).await?;

    let visits = Visits::build(state, path_id).await?;

    let referrers = ReferrerCount::all_sorted_by_count(
        state,
        Some(path_id),
        range.start_datetime(),
        range.end_datetime(),
    )
    .await?;
    let referrers = CountRows::from(referrers);

    let chart = build_chart(state, Some(path_id), &range, now).await?;

    Ok(Stats {
        base: Base::new(state, "Stats"),
        tracked_origin: state.tracked_origin,
        path: path.to_owned(),
        visits,
        referrers,
        chart,
        range,
    })
}
