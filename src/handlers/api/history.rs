use axum::{
    Json,
    extract::{Query, State},
};
use axum_ctx::*;
use serde::{Serialize, Serializer};
use time::PrimitiveDateTime;

use crate::{extractors::query_path::QueryPath, formatters::DateTimeFormatter, states::AppState};

struct Visit {
    registered_at_tz: PrimitiveDateTime,
    referrer: Option<String>,
    time_s: Option<i32>,
}

#[derive(Serialize)]
struct FormattedVisit<'a> {
    registered_at: DateTimeFormatter,
    referrer: &'a Option<String>,
    spent_time_secs: Option<i32>,
}

struct VisitsFormatter(Vec<Visit>);

impl Serialize for VisitsFormatter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.0.iter().map(|visit| FormattedVisit {
            registered_at: DateTimeFormatter(visit.registered_at_tz),
            referrer: &visit.referrer,
            spent_time_secs: visit.time_s,
        }))
    }
}

#[derive(Serialize)]
pub struct History {
    utc_offset: &'static str,
    visits: VisitsFormatter,
}

pub async fn get(
    State(state): AppState,
    Query(path): Query<QueryPath>,
) -> RespResult<Json<History>> {
    let path_id = path.normalized_with_id(&state.pool).await?.path_id;

    let visits = sqlx::query_as!(
        Visit,
        r#"SELECT TIMEZONE($1, registered_at) AS "registered_at_tz!", domain AS "referrer?", time_s FROM visits
        LEFT JOIN referrers ON referrers.id = referrer_id
        WHERE path_id = $2
        ORDER BY "registered_at_tz!""#,
        state.posix_utc_offset_str,
        path_id,
    )
    .fetch_all(&state.pool)
    .await
    .ctx(StatusCode::INTERNAL_SERVER_ERROR)
    .log_msg("History query failed!")?;

    let history = History {
        utc_offset: state.utc_offset_str,
        visits: VisitsFormatter(visits),
    };

    Ok(Json(history))
}
