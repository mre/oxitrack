use axum_ctx::*;
use time::PrimitiveDateTime;

use crate::{handlers::count_rows::Count, states::InnerAppState};

pub struct ReferrerCount {
    pub domain: String,
    pub count: i64,
}

impl ReferrerCount {
    pub async fn all_sorted_by_count(
        state: &'static InnerAppState,
        path_id: i64,
        start_datetime: Option<PrimitiveDateTime>,
    ) -> Result<Vec<Self>, RespErr> {
        sqlx::query_as!(
            Self,
            r#"SELECT domain, COUNT(*) as "count!" FROM visits
            INNER JOIN referrers ON referrers.id = referrer_id
            WHERE path_id = $1 AND ($2::timestamp IS NULL OR TIMEZONE($3, registered_at) >= $2)
            GROUP BY domain
            ORDER BY "count!" DESC"#,
            path_id,
            start_datetime,
            state.posix_utc_offset_str,
        )
        .fetch_all(&state.pool)
        .await
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Failed to query referrers!")
    }
}

impl Count for ReferrerCount {
    fn count(&self) -> i64 {
        self.count
    }
}
