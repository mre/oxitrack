use axum_ctx::{RespErr, RespErrCtx, RespErrExt, Status};
use sqlx::PgPool;
use time::OffsetDateTime;

use super::{OptionStartDateTime, StartDatetime};

pub struct WholeDaysSinceFirstVisit {
    pub whole_days_since_first_visit: i64,
    pub now: OffsetDateTime,
    pub first_visit: OffsetDateTime,
}

impl WholeDaysSinceFirstVisit {
    pub async fn build(
        pool: &PgPool,
        path_id: Option<i64>,
        start_datetime: Option<StartDatetime>,
    ) -> Result<Self, RespErr> {
        let OptionStartDateTime { start, now } = start_datetime.into();

        let first_visit = sqlx::query!(
            "SELECT registered_at FROM visits
            WHERE ($1::bigint IS NULL OR path_id = $1) AND ($2::timestamptz IS NULL OR registered_at > $2)
            ORDER BY registered_at
            LIMIT 1",
            path_id,
            start,
        )
        .fetch_optional(pool)
        .await
        .ctx(Status::Internal)
        .log_msg("Failed to query the first visit!")?
        .ctx(Status::NotFound)
        .user_msg("The requested path has no counted visits yet.")?
        .registered_at;

        let whole_days_since_first_visit = (now - first_visit).whole_days();

        Ok(Self {
            whole_days_since_first_visit,
            now,
            first_visit,
        })
    }
}
