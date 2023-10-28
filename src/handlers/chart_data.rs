pub mod all_time;
mod chart_data_vec;
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

use chart_data_vec::ChartDataVec;
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
        path_id: i64,
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
            WHERE path_id = $2 AND ($3::timestamptz IS NULL OR registered_at > $3)
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
                None => return Ok(Vec::new()),
            },
        };

        let mut chart_data = ChartDataVec::default();

        let now_date_part = D::from(state.apply_utc_offset(now)?);
        let mut iter_date_part = D::from(state.apply_utc_offset(first_datetime)?);

        let last_row_ind = rows.len() - 1;

        for (row_ind, row) in rows.into_iter().enumerate() {
            let row_date_part = D::from(state.apply_utc_offset(row.trunc_registered_at)?);

            if iter_date_part != row_date_part {
                loop {
                    chart_data.push(Self {
                        x: iter_date_part,
                        y: 0,
                    })?;

                    iter_date_part.next()?;

                    if iter_date_part == row_date_part {
                        break;
                    }
                }
            }

            #[allow(clippy::cast_sign_loss)]
            chart_data.push(Self {
                x: iter_date_part,
                y: row.count as u64,
            })?;

            if row_ind != last_row_ind {
                iter_date_part.next()?;
            }
        }

        if iter_date_part != now_date_part {
            iter_date_part.next()?;

            loop {
                chart_data.push(Self {
                    x: iter_date_part,
                    y: 0,
                })?;

                if iter_date_part == now_date_part {
                    break;
                }

                iter_date_part.next()?;
            }
        }

        Ok(chart_data.into_inner())
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
