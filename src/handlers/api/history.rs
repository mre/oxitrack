use axum::{
    extract::{Query, State},
    Json,
};
use axum_ctx::{RespErr, RespErrCtx, RespErrExt, Status};
use serde::{
    ser::{self, SerializeSeq},
    Serialize, Serializer,
};
use time::{OffsetDateTime, UtcOffset};

use crate::{
    extractors::query_path::QueryPath,
    formatters::{DatetimeFormatter, UtcOffsetFormatter},
    states::AppState,
};

struct Visit {
    registered_at: OffsetDateTime,
    referrer: Option<String>,
    left_at: Option<OffsetDateTime>,
}

#[derive(Serialize)]
struct FormattedVisit<'a> {
    registered_at: DatetimeFormatter,
    referrer: &'a Option<String>,
    spent_time_secs: Option<i64>,
}

struct VisitsFormatter {
    utc_offset: UtcOffset,
    visits: Vec<Visit>,
}

impl VisitsFormatter {
    fn apply_utc_offset<S>(&self, datetime: OffsetDateTime) -> Result<DatetimeFormatter, S::Error>
    where
        S: Serializer,
    {
        match datetime.checked_to_offset(self.utc_offset) {
            Some(t) => Ok(DatetimeFormatter(t)),
            None => Err(ser::Error::custom("Failed UTC offset conversion")),
        }
    }
}

impl Serialize for VisitsFormatter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.visits.len()))?;

        for visit in &self.visits {
            let element = FormattedVisit {
                registered_at: self.apply_utc_offset::<S>(visit.registered_at)?,
                referrer: &visit.referrer,
                spent_time_secs: visit
                    .left_at
                    .map(|left_at| (left_at - visit.registered_at).whole_seconds()),
            };
            seq.serialize_element(&element)?;
        }

        seq.end()
    }
}

#[derive(Serialize)]
pub struct History {
    utc_offset: UtcOffsetFormatter,
    visits: VisitsFormatter,
}

pub async fn get(
    State(state): AppState,
    Query(path): Query<QueryPath>,
) -> Result<Json<History>, RespErr> {
    let (path, path_id) = path.normalized_with_id(&state.pool).await?;

    let visits = sqlx::query_as!(
        Visit,
        r#"SELECT registered_at, domain AS "referrer?", left_at FROM visits
        LEFT JOIN referrers ON referrers.id = referrer_id
        WHERE path_id = $1
        ORDER BY registered_at"#,
        path_id,
    )
    .fetch_all(&state.pool)
    .await
    .ctx(Status::Internal)
    .log_msg(|| format!("History query failed for path {path}!"))?;

    let history = History {
        utc_offset: UtcOffsetFormatter(state.utc_offset),
        visits: VisitsFormatter {
            utc_offset: state.utc_offset,
            visits,
        },
    };

    Ok(Json(history))
}
