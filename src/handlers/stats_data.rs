mod chart_data_aggregator;
mod contiguous_date_part;
pub mod referrer_count;
pub mod whole_days_since_first_visit;

pub use whole_days_since_first_visit::WholeDaysSinceFirstVisit;

use axum_ctx::*;
use serde::Deserialize;
use time::{Duration, OffsetDateTime, PrimitiveDateTime, Time};

use crate::{db::Db, states::InnerAppState};

use chart_data_aggregator::ChartDataAggregator;
use contiguous_date_part::{
    ContiguousDatePart, ContiguousDay, ContiguousHour, ContiguousMonth, ContiguousYear,
};

#[derive(sqlx::FromRow)]
struct TruncDateCount {
    trunc_registered_at: PrimitiveDateTime,
    count: i64,
}

/// A single bar in the SVG chart.
pub struct ChartBar {
    pub label: String,
    pub count: u64,
    /// SVG x position
    pub x: f64,
    /// SVG y position (from top, since SVG y increases downward)
    pub y: f64,
    /// Bar width in SVG units
    pub w: f64,
    /// Bar height in SVG units
    pub h: f64,
}

const BAR_WIDTH: f64 = 10.0;
const BAR_PITCH: f64 = 12.0;
const CHART_HEIGHT: f64 = 60.0;

pub fn chart_width(n: usize) -> f64 {
    (n as f64 * BAR_PITCH).max(BAR_PITCH)
}

/// Time filter for chart/stats queries.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Filter {
    Last2Days,
    Last60Days,
    #[default]
    AllTime,
}

impl Filter {
    pub fn label(self) -> &'static str {
        match self {
            Filter::Last2Days => "Last 2 days",
            Filter::Last60Days => "Last 60 days",
            Filter::AllTime => "All time",
        }
    }
}

struct DataPoint<D> {
    x: D,
    y: u64,
}

impl<D> DataPoint<D>
where
    D: ContiguousDatePart,
{
    async fn all(
        state: &InnerAppState,
        path_id: Option<i64>,
        now: OffsetDateTime,
        start_datetime: Option<PrimitiveDateTime>,
        trunc_sql: &str,
    ) -> RespResult<Vec<Self>> {
        let sql = format!(
            r#"SELECT {trunc_sql} AS trunc_registered_at,
            COUNT(registered_at) AS count FROM visits
            WHERE (? IS NULL OR path_id = ?) AND (? IS NULL OR datetime(registered_at, ?) >= datetime(?))
            GROUP BY trunc_registered_at
            ORDER BY trunc_registered_at"#
        );

        let rows = sqlx::query_as::<Db, TruncDateCount>(&sql)
            .bind(path_id)
            .bind(path_id)
            .bind(start_datetime)
            .bind(state.posix_utc_offset_str)
            .bind(start_datetime)
            .fetch_all(&state.pool)
            .await
            .ctx(StatusCode::INTERNAL_SERVER_ERROR)
            .log_msg("Failed to query chart data!")?;

        let now_date_part = D::from(now);

        let first_date_part = if let Some(start_datetime) = start_datetime {
            D::from(start_datetime)
        } else if let Some(row) = rows.first() {
            D::from(row.trunc_registered_at)
        } else {
            return Ok(vec![Self {
                x: now_date_part,
                y: 0,
            }]);
        };

        let additional_given_point = match rows.last() {
            Some(last_row) => {
                let last_row_date_part = D::from(last_row.trunc_registered_at);
                (now_date_part > last_row_date_part).then_some(Ok((now_date_part, 0)))
            }
            None => Some(Ok((now_date_part, 0))),
        };

        #[allow(clippy::cast_sign_loss)]
        let given_points = rows
            .into_iter()
            .map(|row| {
                let row_date_part = D::from(row.trunc_registered_at);
                Ok((row_date_part, row.count as u64))
            })
            .chain(additional_given_point);

        let mut aggregator = ChartDataAggregator::new(first_date_part);

        for given_point in given_points {
            let (row_date_part, count) = given_point?;

            if aggregator.next_date_part() < row_date_part {
                loop {
                    aggregator.push(0)?;
                    if aggregator.next_date_part() >= row_date_part {
                        break;
                    }
                }
            }
            aggregator.push(count)?;
        }

        Ok(aggregator.into_inner())
    }
}

fn to_chart_bars<D: ContiguousDatePart + std::fmt::Display>(
    points: Vec<DataPoint<D>>,
) -> Vec<ChartBar> {
    let max_count = points.iter().map(|p| p.y).max().unwrap_or(1).max(1);
    points
        .into_iter()
        .enumerate()
        .map(|(i, p)| {
            let h =
                (p.y as f64 / max_count as f64 * CHART_HEIGHT).max(if p.y > 0 { 1.0 } else { 0.0 });
            ChartBar {
                label: p.x.to_string(),
                count: p.y,
                x: i as f64 * BAR_PITCH,
                y: CHART_HEIGHT - h,
                w: BAR_WIDTH,
                h,
            }
        })
        .collect()
}

pub async fn build_chart(
    state: &'static InnerAppState,
    path_id: Option<i64>,
    filter: Filter,
) -> RespResult<Vec<ChartBar>> {
    let now = state.now_tz()?;

    match filter {
        Filter::Last2Days => {
            let start = hour_data_start_datetime(now)?;
            let trunc = format!(
                "strftime('%Y-%m-%d %H:00:00', datetime(registered_at, '{}'))",
                state.posix_utc_offset_str
            );
            let points =
                DataPoint::<ContiguousHour>::all(state, path_id, now, Some(start), &trunc).await?;
            Ok(to_chart_bars(points))
        }
        Filter::Last60Days => {
            let start = day_data_start_datetime(now);
            let trunc = format!(
                "strftime('%Y-%m-%d 00:00:00', datetime(registered_at, '{}'))",
                state.posix_utc_offset_str
            );
            let points =
                DataPoint::<ContiguousDay>::all(state, path_id, now, Some(start), &trunc).await?;
            Ok(to_chart_bars(points))
        }
        Filter::AllTime => {
            let Some(WholeDaysSinceFirstVisit {
                whole_days_since_first_visit,
                ..
            }) = WholeDaysSinceFirstVisit::build(state, path_id, now, None).await?
            else {
                return Ok(vec![]);
            };

            if whole_days_since_first_visit < 2 {
                let start = hour_data_start_datetime(now)?;
                let trunc = format!(
                    "strftime('%Y-%m-%d %H:00:00', datetime(registered_at, '{}'))",
                    state.posix_utc_offset_str
                );
                let points =
                    DataPoint::<ContiguousHour>::all(state, path_id, now, Some(start), &trunc)
                        .await?;
                Ok(to_chart_bars(points))
            } else if whole_days_since_first_visit < 60 {
                let start = day_data_start_datetime(now);
                let trunc = format!(
                    "strftime('%Y-%m-%d 00:00:00', datetime(registered_at, '{}'))",
                    state.posix_utc_offset_str
                );
                let points =
                    DataPoint::<ContiguousDay>::all(state, path_id, now, Some(start), &trunc)
                        .await?;
                Ok(to_chart_bars(points))
            } else if whole_days_since_first_visit < 1461 {
                let trunc = format!(
                    "strftime('%Y-%m-01 00:00:00', datetime(registered_at, '{}'))",
                    state.posix_utc_offset_str
                );
                let points =
                    DataPoint::<ContiguousMonth>::all(state, path_id, now, None, &trunc).await?;
                Ok(to_chart_bars(points))
            } else {
                let trunc = format!(
                    "strftime('%Y-01-01 00:00:00', datetime(registered_at, '{}'))",
                    state.posix_utc_offset_str
                );
                let points =
                    DataPoint::<ContiguousYear>::all(state, path_id, now, None, &trunc).await?;
                Ok(to_chart_bars(points))
            }
        }
    }
}

pub fn start_datetime_for_filter(
    filter: Filter,
    now: OffsetDateTime,
) -> RespResult<Option<PrimitiveDateTime>> {
    match filter {
        Filter::Last2Days => Ok(Some(hour_data_start_datetime(now)?)),
        Filter::Last60Days => Ok(Some(day_data_start_datetime(now))),
        Filter::AllTime => Ok(None),
    }
}

fn hour_data_start_datetime(now: OffsetDateTime) -> RespResult<PrimitiveDateTime> {
    let date = now.date() - Duration::days(2);
    let time = Time::from_hms(now.hour(), 0, 0)
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Failed to create Time for hour data!")?;
    Ok(PrimitiveDateTime::new(date, time))
}

fn day_data_start_datetime(now: OffsetDateTime) -> PrimitiveDateTime {
    PrimitiveDateTime::new(now.date() - Duration::days(59), Time::MIDNIGHT)
}
