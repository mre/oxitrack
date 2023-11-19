use axum_ctx::{RespErr, RespErrCtx, RespErrExt, Status};
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
    ) -> Result<Option<Self>, RespErr> {
        let first_visit = sqlx::query!(
            "SELECT registered_at FROM visits
            WHERE ($1::bigint IS NULL OR path_id = $1) AND ($2::timestamp IS NULL OR TIMEZONE($3, registered_at) >= $2)
            ORDER BY registered_at
            LIMIT 1",
            path_id,
            start_datetime,
            state.posix_utc_offset_str,
        )
        .fetch_optional(&state.pool)
        .await
        .ctx(Status::Internal)
        .log_msg("Failed to query the first visit!")?;

        let first_visit = match first_visit {
            Some(v) => v.registered_at,
            None => return Ok(None),
        };

        let whole_days_since_first_visit = (now - first_visit).whole_days();

        Ok(Some(Self {
            whole_days_since_first_visit,
            first_visit,
        }))
    }
}
