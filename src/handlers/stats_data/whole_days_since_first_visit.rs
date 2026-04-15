use axum_ctx::*;
use sqlx::Row;
use time::{OffsetDateTime, PrimitiveDateTime};

use crate::states::InnerAppState;

pub struct WholeDaysSinceFirstVisit {
    pub whole_days_since_first_visit: i64,
    pub first_visit: OffsetDateTime,
}

impl WholeDaysSinceFirstVisit {
    pub async fn build(
        state: &'static InnerAppState,
        path_id: Option<i64>,
        now: OffsetDateTime,
        start_datetime: Option<PrimitiveDateTime>,
    ) -> RespResult<Option<Self>> {
        // PostgreSQL: $N is positional – the same slot is reused for each $1/$2.
        #[cfg(feature = "postgres")]
        let first_visit_row = sqlx::query(
            "SELECT registered_at FROM visits
            WHERE ($1 IS NULL OR path_id = $1)
              AND ($2 IS NULL OR TIMEZONE($3, registered_at) >= $2)
            ORDER BY registered_at
            LIMIT 1",
        )
        .bind(path_id)
        .bind(start_datetime)
        .bind(state.posix_utc_offset_str)
        .fetch_optional(&state.pool)
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Failed to query the first visit!")?;

        // SQLite: each `?` is a distinct positional slot, so repeated values
        // must each be bound separately.
        #[cfg(feature = "sqlite")]
        let first_visit_row = sqlx::query(
            "SELECT registered_at FROM visits
            WHERE (? IS NULL OR path_id = ?)
              AND (? IS NULL OR datetime(registered_at, ?) >= datetime(?))
            ORDER BY registered_at
            LIMIT 1",
        )
        .bind(path_id)
        .bind(path_id)
        .bind(start_datetime)
        .bind(state.posix_utc_offset_str)
        .bind(start_datetime)
        .fetch_optional(&state.pool)
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Failed to query the first visit!")?;

        let first_visit: OffsetDateTime = match first_visit_row {
            Some(row) => row.get("registered_at"),
            None => return Ok(None),
        };

        let whole_days_since_first_visit = (now - first_visit).whole_days();

        Ok(Some(Self {
            whole_days_since_first_visit,
            first_visit,
        }))
    }
}
