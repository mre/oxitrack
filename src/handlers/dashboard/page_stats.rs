use axum_ctx::*;
use time::OffsetDateTime;

use crate::{
    db::Db,
    formatters::SecondsFormatter,
    handlers::{count_rows::Count, stats_data::DateRange},
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
    first_registered_at: Option<time::PrimitiveDateTime>,
}

pub async fn all_sorted_by_count(
    state: &'static InnerAppState,
    range: &DateRange,
    now: OffsetDateTime,
) -> RespResult<Vec<PageStat>> {
    let start_datetime = range.start_datetime();
    let end_datetime = range.end_datetime();

    let rows = sqlx::query_as::<Db, PageStatRow>(
        r#"SELECT paths.path,
            COUNT(*) AS count,
            AVG(visits.time_s) AS avg_time_s,
            MIN(visits.registered_at) AS first_registered_at
        FROM paths
        INNER JOIN visits ON visits.path_id = paths.id
        WHERE (? IS NULL OR datetime(visits.registered_at, ?) >= datetime(?))
          AND (? IS NULL OR datetime(visits.registered_at, ?) < datetime(?))
        GROUP BY paths.path
        ORDER BY count DESC"#,
    )
    .bind(start_datetime)
    .bind(state.posix_utc_offset_str)
    .bind(start_datetime)
    .bind(end_datetime)
    .bind(state.posix_utc_offset_str)
    .bind(end_datetime)
    .fetch_all(&state.pool)
    .await
    .ctx(StatusCode::INTERNAL_SERVER_ERROR)
    .log_msg("Page stats query failed!")?;

    Ok(rows
        .into_iter()
        .map(|row| {
            #[allow(clippy::cast_precision_loss)]
            let days = range.whole_days(now).unwrap_or_else(|| {
                row.first_registered_at
                    .map(|fv| (now.date() - fv.date()).whole_days().max(1))
                    .unwrap_or(1)
            }) as f64;
            let per_day = row.count as f64 / days;

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
