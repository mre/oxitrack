use axum_ctx::{RespErrCtx, RespErrExt, RespResult, StatusCode};
use sqlx::Row;
use time::OffsetDateTime;

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
    ) -> RespResult<Option<Self>> {
        let first_visit_row = sqlx::query(
            "SELECT registered_at FROM visits
            WHERE (? IS NULL OR path_id = ?)
            ORDER BY registered_at
            LIMIT 1",
        )
        .bind(path_id)
        .bind(path_id)
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
