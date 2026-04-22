use axum_ctx::*;
use time::{OffsetDateTime, PrimitiveDateTime};

use crate::{
    db::Db,
    formatters::SecondsFormatter,
    handlers::{count_rows::Count, stats_data::Filter},
    states::InnerAppState,
};

pub struct PageStat {
    pub path: String,
    pub count: i64,
    pub avg_duration: Option<SecondsFormatter>,
    pub per_day: f64,
}

impl Count for PageStat {
    #[inline]
    fn count(&self) -> i64 {
        self.count
    }
}

#[derive(sqlx::FromRow)]
struct PageStatRow {
    path: String,
    count: i64,
    avg_time_s: Option<f64>,
    first_registered_at: Option<PrimitiveDateTime>,
}

pub async fn all_sorted_by_count(
    state: &'static InnerAppState,
    filter: Filter,
    now: OffsetDateTime,
    start_datetime: Option<PrimitiveDateTime>,
) -> RespResult<Vec<PageStat>> {
    let rows = sqlx::query_as::<Db, PageStatRow>(
        r#"SELECT paths.path,
            COUNT(*) AS count,
            AVG(visits.time_s) AS avg_time_s,
            MIN(visits.registered_at) AS first_registered_at
        FROM paths
        INNER JOIN visits ON visits.path_id = paths.id
        WHERE ? IS NULL OR datetime(visits.registered_at, ?) >= datetime(?)
        GROUP BY paths.path
        ORDER BY count DESC"#,
    )
    .bind(start_datetime)
    .bind(state.posix_utc_offset_str)
    .bind(start_datetime)
    .fetch_all(&state.pool)
    .await
    .ctx(StatusCode::INTERNAL_SERVER_ERROR)
    .log_msg("Page stats query failed!")?;

    let fixed_denom: Option<f64> = match filter {
        Filter::Last2Days => Some(2.0),
        Filter::Last60Days => Some(60.0),
        Filter::AllTime => None,
    };

    Ok(rows
        .into_iter()
        .map(|row| {
            #[allow(clippy::cast_precision_loss)]
            let per_day = if let Some(denom) = fixed_denom {
                row.count as f64 / denom
            } else {
                let days = row
                    .first_registered_at
                    .map(|fv| (now.date() - fv.date()).whole_days().max(1))
                    .unwrap_or(1);
                row.count as f64 / days as f64
            };

            PageStat {
                path: row.path,
                count: row.count,
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                avg_duration: row.avg_time_s.map(|s| SecondsFormatter(s as u64)),
                per_day,
            }
        })
        .collect())
}
