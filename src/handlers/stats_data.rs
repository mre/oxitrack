mod chart_data_aggregator;
mod contiguous_date_part;
pub mod referrer_count;
pub mod whole_days_since_first_visit;

pub use whole_days_since_first_visit::WholeDaysSinceFirstVisit;

use axum_ctx::{RespErrCtx, RespErrExt, RespResult, StatusCode};
use time::macros::format_description;
use time::{Date, Duration, OffsetDateTime, PrimitiveDateTime, Time};

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

/// A single bar in the chart.
pub struct ChartBar {
    pub label: String,
    pub count: u64,
}

/// Arbitrary date range filter for chart/stats queries.
#[derive(Clone, Default)]
pub struct DateRange {
    pub from: Option<Date>,
    pub to: Option<Date>,
}

impl DateRange {
    pub fn from_params(from: Option<String>, to: Option<String>) -> Self {
        let fmt = format_description!("[year]-[month]-[day]");
        Self {
            from: from
                .filter(|s| !s.is_empty())
                .and_then(|s| Date::parse(&s, fmt).ok()),
            to: to
                .filter(|s| !s.is_empty())
                .and_then(|s| Date::parse(&s, fmt).ok()),
        }
    }

    pub fn start_datetime(&self) -> Option<PrimitiveDateTime> {
        self.from.map(|d| PrimitiveDateTime::new(d, Time::MIDNIGHT))
    }

    pub fn end_datetime(&self) -> Option<PrimitiveDateTime> {
        self.to
            .map(|d| PrimitiveDateTime::new(d + Duration::days(1), Time::MIDNIGHT))
    }

    pub fn whole_days(&self, now: OffsetDateTime) -> Option<i64> {
        let from = self.from?;
        let to = self.to.unwrap_or_else(|| now.date());
        Some((to - from).whole_days().max(1))
    }

    pub fn label(&self) -> String {
        // Try to identify common presets and give them a friendly name.
        // The "today" preset sends from=today&to=today (0-day span).
        // Other presets send from=N-days-ago&to=today (N-day span).
        if let (Some(from), Some(to)) = (self.from, self.to) {
            let days = (to - from).whole_days();
            match days {
                0 => return "Today".to_string(),
                7 => return "Last 7 days".to_string(),
                30 => return "Last 30 days".to_string(),
                90 => return "Last 90 days".to_string(),
                365 => return "Last year".to_string(),
                _ => {}
            }
        }

        let fmt = format_description!("[year]-[month]-[day]");
        match (self.from, self.to) {
            (None, _) => "All time".to_string(),
            (Some(f), None) => format!("Since {}", f.format(fmt).unwrap_or_default()),
            (Some(f), Some(t)) => format!(
                "{} – {}",
                f.format(fmt).unwrap_or_default(),
                t.format(fmt).unwrap_or_default()
            ),
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn from_input(&self) -> String {
        let fmt = format_description!("[year]-[month]-[day]");
        self.from
            .and_then(|d| d.format(fmt).ok())
            .unwrap_or_default()
    }

    pub fn to_input(&self) -> String {
        let fmt = format_description!("[year]-[month]-[day]");
        self.to.and_then(|d| d.format(fmt).ok()).unwrap_or_default()
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
        end_datetime: Option<PrimitiveDateTime>,
        trunc_sql: &str,
    ) -> RespResult<Vec<Self>> {
        let start_utc = start_datetime.map(|pdt| local_to_utc(pdt, state.utc_offset));
        let end_utc = end_datetime.map(|pdt| local_to_utc(pdt, state.utc_offset));

        let sql = format!(
            r"SELECT {trunc_sql} AS trunc_registered_at,
            COUNT(registered_at) AS count FROM visits
            WHERE (? IS NULL OR path_id = ?)
              AND (? IS NULL OR registered_at >= ?)
              AND (? IS NULL OR registered_at < ?)
            GROUP BY trunc_registered_at
            ORDER BY trunc_registered_at"
        );

        let rows = sqlx::query_as::<Db, TruncDateCount>(&sql)
            .bind(path_id)
            .bind(path_id)
            .bind(start_utc)
            .bind(start_utc)
            .bind(end_utc)
            .bind(end_utc)
            .fetch_all(&state.pool)
            .await
            .ctx(StatusCode::INTERNAL_SERVER_ERROR)
            .log_msg("Failed to query chart data!")?;

        let now_date_part = D::from(now);
        let terminal_date_part = end_datetime.map(D::from).map_or(now_date_part, |ep| {
            if ep < now_date_part {
                ep
            } else {
                now_date_part
            }
        });

        let first_date_part = if let Some(start_datetime) = start_datetime {
            D::from(start_datetime)
        } else if let Some(row) = rows.first() {
            D::from(row.trunc_registered_at)
        } else {
            return Ok(vec![Self {
                x: terminal_date_part,
                y: 0,
            }]);
        };

        #[allow(clippy::option_if_let_else)]
        let additional_given_point = match rows.last() {
            Some(last_row) => {
                let last_row_date_part = D::from(last_row.trunc_registered_at);
                (terminal_date_part > last_row_date_part).then_some(Ok((terminal_date_part, 0)))
            }
            None => Some(Ok((terminal_date_part, 0))),
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

/// Converts a local-time `PrimitiveDateTime` to UTC by subtracting the UTC offset.
/// This is the inverse of `InnerAppState::apply_utc_offset`.
pub fn local_to_utc(pdt: PrimitiveDateTime, offset: time::UtcOffset) -> PrimitiveDateTime {
    pdt - time::Duration::seconds(i64::from(offset.whole_seconds()))
}

fn to_chart_bars<D: ContiguousDatePart + std::fmt::Display>(
    points: Vec<DataPoint<D>>,
) -> Vec<ChartBar> {
    points
        .into_iter()
        .map(|p| ChartBar {
            label: p.x.to_string(),
            count: p.y,
        })
        .collect()
}

pub async fn build_chart(
    state: &'static InnerAppState,
    path_id: Option<i64>,
    range: &DateRange,
    now: OffsetDateTime,
) -> RespResult<Vec<ChartBar>> {
    let start_dt = range.start_datetime();
    let end_dt = range.end_datetime();

    let whole_days = if let Some(days) = range.whole_days(now) {
        days
    } else {
        let Some(WholeDaysSinceFirstVisit {
            whole_days_since_first_visit,
            ..
        }) = WholeDaysSinceFirstVisit::build(state, path_id, now).await?
        else {
            return Ok(vec![]);
        };
        whole_days_since_first_visit
    };

    if whole_days < 3 {
        let start = if start_dt.is_none() {
            Some(hour_data_start_datetime(now)?)
        } else {
            start_dt
        };
        let trunc = format!(
            "strftime('%Y-%m-%d %H:00:00', datetime(registered_at, '{}'))",
            state.posix_utc_offset_str
        );
        let points =
            DataPoint::<ContiguousHour>::all(state, path_id, now, start, end_dt, &trunc).await?;
        Ok(to_chart_bars(points))
    } else if whole_days < 91 {
        let trunc = format!(
            "strftime('%Y-%m-%d 00:00:00', datetime(registered_at, '{}'))",
            state.posix_utc_offset_str
        );
        let points =
            DataPoint::<ContiguousDay>::all(state, path_id, now, start_dt, end_dt, &trunc).await?;
        Ok(to_chart_bars(points))
    } else if whole_days < 3653 {
        let trunc = format!(
            "strftime('%Y-%m-01 00:00:00', datetime(registered_at, '{}'))",
            state.posix_utc_offset_str
        );
        let points =
            DataPoint::<ContiguousMonth>::all(state, path_id, now, start_dt, end_dt, &trunc)
                .await?;
        Ok(to_chart_bars(points))
    } else {
        let trunc = format!(
            "strftime('%Y-01-01 00:00:00', datetime(registered_at, '{}'))",
            state.posix_utc_offset_str
        );
        let points =
            DataPoint::<ContiguousYear>::all(state, path_id, now, start_dt, end_dt, &trunc).await?;
        Ok(to_chart_bars(points))
    }
}

fn hour_data_start_datetime(now: OffsetDateTime) -> RespResult<PrimitiveDateTime> {
    let date = now.date() - Duration::days(2);
    let time = Time::from_hms(now.hour(), 0, 0)
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Failed to create Time for hour data!")?;
    Ok(PrimitiveDateTime::new(date, time))
}
