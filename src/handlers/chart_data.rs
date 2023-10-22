pub mod all_time;
mod contiguous_date_part;
pub mod last_2_days;
pub mod last_60_days;

use axum::Json;
use axum_ctx::{RespErr, RespErrCtx, RespErrExt, Status};
use serde::Serialize;
use sqlx::PgPool;
use std::num::NonZeroU64;
use time::{Duration, OffsetDateTime};

use contiguous_date_part::ContiguousDatePart;

use crate::states::InnerAppState;

struct TruncDateCount {
    trunc_registered_at: OffsetDateTime,
    count: i64,
}

#[derive(Clone)]
pub struct StartDatetime {
    start: OffsetDateTime,
    now: OffsetDateTime,
}

impl StartDatetime {
    pub fn from_sub_duration(duration: Duration) -> Self {
        let now = OffsetDateTime::now_utc();

        Self {
            start: now - duration,
            now,
        }
    }
}

pub struct OptionStartDateTime {
    pub start: Option<OffsetDateTime>,
    pub now: OffsetDateTime,
}

impl From<Option<StartDatetime>> for OptionStartDateTime {
    fn from(opt: Option<StartDatetime>) -> Self {
        match opt {
            Some(StartDatetime { start, now }) => Self {
                start: Some(start),
                now,
            },
            None => Self {
                start: None,
                now: OffsetDateTime::now_utc(),
            },
        }
    }
}

#[derive(Serialize)]
pub struct DataPoint {
    x: String,
    y: u64,
}

impl DataPoint {
    async fn all<D: ContiguousDatePart>(
        state: &InnerAppState,
        path_id: i64,
        start_datetime: Option<StartDatetime>,
    ) -> Result<Json<Vec<Self>>, RespErr> {
        let date_truncation = D::date_truncation();

        let OptionStartDateTime { start, now } = start_datetime.into();

        // Warning: The rows are assumed to be sorted after the registration date.
        // Unsorted rows can lead to an endless loop below.
        let rows = sqlx::query_as!(
            TruncDateCount,
            r#"SELECT date_trunc($1, registered_at) AS "trunc_registered_at!",
            COUNT(registered_at) AS "count!" FROM visits
            WHERE path_id = $2 AND ($3::timestamptz IS NULL OR registered_at > $3)
            GROUP BY "trunc_registered_at!"
            ORDER BY "trunc_registered_at!""#,
            date_truncation,
            path_id,
            start,
        )
        .fetch_all(&state.pool)
        .await
        .ctx(Status::Internal)
        .log_msg("Failed to query chart data!")?;

        let (first_date, last_date) = match rows.as_slice() {
            [] => return Ok(Json(Vec::new())),
            [single] => (single.trunc_registered_at, single.trunc_registered_at),
            [first, .., last] => (first.trunc_registered_at, last.trunc_registered_at),
        };

        let mut chart_data = Vec::with_capacity(rows.len());

        let now_date_part = D::from(state.apply_utc_offset(now)?);
        let mut iter_date_part = D::from(state.apply_utc_offset(first_date)?);

        for row in rows {
            let row_date_part = D::from(state.apply_utc_offset(row.trunc_registered_at)?);

            if iter_date_part == row_date_part {
                chart_data.push(DataPoint {
                    x: iter_date_part.to_string(),
                    y: row.count as u64,
                });

                iter_date_part.next()?;

                continue;
            }

            loop {
                chart_data.push(DataPoint {
                    x: iter_date_part.to_string(),
                    y: 0,
                });

                iter_date_part.next()?;

                if iter_date_part == row_date_part {
                    chart_data.push(DataPoint {
                        x: iter_date_part.to_string(),
                        y: row.count as u64,
                    });

                    iter_date_part.next()?;

                    break;
                }
            }
        }

        if now_date_part != D::from(state.apply_utc_offset(last_date)?) {
            loop {
                chart_data.push(DataPoint {
                    x: iter_date_part.to_string(),
                    y: 0,
                });

                if iter_date_part == now_date_part {
                    break;
                }

                iter_date_part.next()?;
            }
        }

        Ok(Json(chart_data))
    }
}

pub struct TotalLen(NonZeroU64);

impl TotalLen {
    pub async fn build(pool: &PgPool, path_id: i64) -> Result<Self, RespErr> {
        let len = sqlx::query!(
            r#"SELECT COUNT(*) AS "count!" FROM visits
            WHERE path_id = $1"#,
            path_id,
        )
        .fetch_one(pool)
        .await
        .ctx(Status::Internal)
        .log_msg("Failed to query the count of visits")?
        .count as u64;

        match NonZeroU64::new(len) {
            Some(len) => Ok(Self(len)),
            None => Err(RespErr::new(Status::NotFound)
                .user_msg("The requested path has no counted visits yet.")),
        }
    }

    #[inline]
    #[must_use]
    pub fn inner(&self) -> NonZeroU64 {
        self.0
    }
}

pub struct DaysSinceFirstVisit {
    pub now: OffsetDateTime,
    pub first_visit: OffsetDateTime,
    pub days_since_first_visit: i64,
}

impl DaysSinceFirstVisit {
    pub async fn build(
        pool: &PgPool,
        path_id: i64,
        start_datetime: Option<StartDatetime>,
    ) -> Result<Self, RespErr> {
        let OptionStartDateTime { start, now } = start_datetime.into();

        let first_visit = sqlx::query!(
            "SELECT registered_at FROM visits
            WHERE path_id = $1 AND ($2::timestamptz IS NULL OR registered_at > $2)
            ORDER BY registered_at
            LIMIT 1",
            path_id,
            start,
        )
        .fetch_optional(pool)
        .await
        .ctx(Status::Internal)
        .user_msg("Failed to query the first visit")?
        .ctx(Status::NotFound)
        .user_msg("The requested path has no counted visits yet.")?
        .registered_at;

        let days_since_first_visit = (now - first_visit).whole_days();

        Ok(Self {
            now,
            first_visit,
            days_since_first_visit,
        })
    }
}
