pub mod all_time;
mod chart_data_aggregator;
mod contiguous_date_part;
pub mod last_2_days;
pub mod last_60_days;
mod start_datetime;
mod whole_days_since_first_visit;

pub use start_datetime::{OptionStartDateTime, StartDatetime};
pub use whole_days_since_first_visit::WholeDaysSinceFirstVisit;

use axum_ctx::{RespErr, RespErrCtx, RespErrExt, Status};
use serde::Serialize;
use time::OffsetDateTime;

use crate::states::InnerAppState;

use chart_data_aggregator::ChartDataAggregator;
use contiguous_date_part::{
    ContiguousDatePart, ContiguousDay, ContiguousHour, ContiguousMonth, ContiguousYear,
};

struct TruncDateCount {
    trunc_registered_at: OffsetDateTime,
    count: i64,
}

#[derive(Serialize)]
pub struct DataPoint<T>
where
    T: Serialize,
{
    x: T,
    y: u64,
}

impl<D> DataPoint<D>
where
    D: ContiguousDatePart,
{
    async fn all(
        state: &InnerAppState,
        path_id: Option<i64>,
        start_datetime: Option<StartDatetime>,
    ) -> Result<Vec<Self>, RespErr> {
        let OptionStartDateTime {
            start: start_datetime,
            now,
        } = start_datetime.into();

        let rows = sqlx::query_as!(
            TruncDateCount,
            r#"SELECT date_trunc($1, registered_at) AS "trunc_registered_at!",
            COUNT(registered_at) AS "count!" FROM visits
            WHERE ($2::bigint IS NULL OR path_id = $2) AND ($3::timestamptz IS NULL OR registered_at > $3)
            GROUP BY "trunc_registered_at!"
            ORDER BY "trunc_registered_at!""#,
            D::date_truncation(),
            path_id,
            start_datetime,
        )
        .fetch_all(&state.pool)
        .await
        .ctx(Status::Internal)
        .log_msg("Failed to query chart data!")?;

        let first_datetime = match start_datetime {
            Some(v) => v,
            None => match rows.first() {
                Some(row) => row.trunc_registered_at,
                // all-time without any rows.
                None => return Ok(Vec::new()),
            },
        };

        // Fill from the last row until now if the date part of the last row is not now.
        let now_date_part = D::from(state.apply_utc_offset(now)?);
        let additional_now_row = match rows.last() {
            Some(last_row) => {
                let last_row_date_part =
                    D::from(state.apply_utc_offset(last_row.trunc_registered_at)?);

                (now_date_part != last_row_date_part).then_some(Ok((now_date_part, 0)))
            }
            None => Some(Ok((now_date_part, 0))),
        };

        #[allow(clippy::cast_sign_loss)]
        let rows_iter = rows
            .into_iter()
            .map(|row| {
                let row_date_part = D::from(state.apply_utc_offset(row.trunc_registered_at)?);
                Ok((row_date_part, row.count as u64))
            })
            .chain(additional_now_row);

        let mut aggregator = {
            let first_date_part = D::from(state.apply_utc_offset(first_datetime)?);
            ChartDataAggregator::new(first_date_part)
        };

        for result in rows_iter {
            let (row_date_part, count) = result?;

            // Fill the gap until the row.
            if aggregator.next_date_part() != row_date_part {
                loop {
                    aggregator.push(0)?;

                    if aggregator.next_date_part() == row_date_part {
                        break;
                    }
                }
            }

            // Add row.
            aggregator.push(count)?;
        }

        Ok(aggregator.into_inner())
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum ChartData {
    Year(Vec<DataPoint<ContiguousYear>>),
    Month(Vec<DataPoint<ContiguousMonth>>),
    Day(Vec<DataPoint<ContiguousDay>>),
    Hour(Vec<DataPoint<ContiguousHour>>),
}
