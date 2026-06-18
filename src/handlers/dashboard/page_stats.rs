use axum_ctx::{RespErrCtx, RespErrExt, RespResult, StatusCode};
use time::OffsetDateTime;

use crate::{
    db::Db,
    formatters::SecondsFormatter,
    handlers::{
        count_rows::Count,
        stats_data::{DateRange, local_to_utc},
    },
    states::InnerAppState,
};

pub struct PageStat {
    pub path: String,
    pub count: i64,
    pub avg_duration: Option<SecondsFormatter>,
    pub per_day: f64,
    pub trend: Trend,
}

/// Direction traffic moved across the active date range, derived by comparing
/// the number of visits in the first half of the range against the second half.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    Up,
    Down,
    Flat,
}

impl Trend {
    /// Arrow glyph shown next to the visits/day value.
    #[inline]
    pub const fn arrow(self) -> &'static str {
        match self {
            Self::Up => "\u{25B2}",   // ▲
            Self::Down => "\u{25BC}", // ▼
            Self::Flat => "",
        }
    }

    /// CSS class used to colour the arrow.
    #[inline]
    pub const fn class(self) -> &'static str {
        match self {
            Self::Up => "trend-up",
            Self::Down => "trend-down",
            Self::Flat => "trend-flat",
        }
    }

    /// Accessible label describing the movement.
    #[inline]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Up => "Traffic grew over this range",
            Self::Down => "Traffic fell over this range",
            Self::Flat => "Traffic held steady over this range",
        }
    }
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
    first_half: i64,
    second_half: i64,
}

pub async fn all_sorted_by_count(
    state: &'static InnerAppState,
    range: &DateRange,
    now: OffsetDateTime,
) -> RespResult<Vec<PageStat>> {
    let start_datetime = range.start_datetime();
    let end_datetime = range.end_datetime();

    let start_utc = start_datetime.map(|pdt| local_to_utc(pdt, state.utc_offset));
    let end_utc = end_datetime.map(|pdt| local_to_utc(pdt, state.utc_offset));

    // Split the active range at its midpoint so we can compare visit volume in
    // the earlier half against the later half and derive a per-path trend.
    // Open-ended ranges fall back to "now" for the end; a missing start (e.g.
    // "All time") leaves the midpoint undefined and yields a flat trend.
    let now_utc = local_to_utc(
        time::PrimitiveDateTime::new(now.date(), now.time()),
        state.utc_offset,
    );
    let effective_end = end_utc.unwrap_or(now_utc);
    let midpoint = start_utc.map(|start| start + (effective_end - start) / 2);

    let rows = sqlx::query_as::<Db, PageStatRow>(
        r"SELECT paths.path,
            COUNT(*) AS count,
            AVG(visits.time_s) AS avg_time_s,
            MIN(visits.registered_at) AS first_registered_at,
            SUM(CASE WHEN ? IS NOT NULL AND visits.registered_at < ? THEN 1 ELSE 0 END) AS first_half,
            SUM(CASE WHEN ? IS NOT NULL AND visits.registered_at >= ? THEN 1 ELSE 0 END) AS second_half
        FROM paths
        INNER JOIN visits ON visits.path_id = paths.id
        WHERE (? IS NULL OR visits.registered_at >= ?)
          AND (? IS NULL OR visits.registered_at < ?)
        GROUP BY paths.path
        ORDER BY count DESC",
    )
    .bind(midpoint)
    .bind(midpoint)
    .bind(midpoint)
    .bind(midpoint)
    .bind(start_utc)
    .bind(start_utc)
    .bind(end_utc)
    .bind(end_utc)
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
                    .map_or(1, |fv| (now.date() - fv.date()).whole_days().max(1))
            }) as f64;
            #[allow(clippy::cast_precision_loss)]
            let per_day = row.count as f64 / days;

            PageStat {
                path: row.path,
                count: row.count,
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                avg_duration: row.avg_time_s.map(|s| SecondsFormatter(s as u64)),
                per_day,
                trend: match row.second_half.cmp(&row.first_half) {
                    std::cmp::Ordering::Greater => Trend::Up,
                    std::cmp::Ordering::Less => Trend::Down,
                    std::cmp::Ordering::Equal => Trend::Flat,
                },
            }
        })
        .collect())
}
