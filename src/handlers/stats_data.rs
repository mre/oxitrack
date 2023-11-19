pub mod all_time;
mod chart_data_aggregator;
mod contiguous_date_part;
pub mod last_2_days;
pub mod last_60_days;
pub mod referrer_count;
pub mod table_body_templates;
mod whole_days_since_first_visit;

use askama::Template;
use axum::Json;
pub use whole_days_since_first_visit::WholeDaysSinceFirstVisit;

use axum_ctx::{RespErr, RespErrCtx, RespErrExt, Status};
use serde::Serialize;
use time::{OffsetDateTime, PrimitiveDateTime};

use crate::{db::VisitCount, states::InnerAppState};

use chart_data_aggregator::ChartDataAggregator;
use contiguous_date_part::{
    ContiguousDatePart, ContiguousDay, ContiguousHour, ContiguousMonth, ContiguousYear,
};

use self::{
    referrer_count::ReferrerCount,
    table_body_templates::{ReferrersTableBody, VisitsTableBody},
};

use super::count_rows::CountRows;

struct TruncDateCount {
    trunc_registered_at: PrimitiveDateTime,
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
        now: OffsetDateTime,
        start_datetime: Option<PrimitiveDateTime>,
    ) -> Result<Vec<Self>, RespErr> {
        let rows = sqlx::query_as!(
            TruncDateCount,
            r#"SELECT DATE_TRUNC($1, TIMEZONE($4, registered_at)) AS "trunc_registered_at!",
            COUNT(registered_at) AS "count!" FROM visits
            WHERE ($2::bigint IS NULL OR path_id = $2) AND ($3::timestamp IS NULL OR TIMEZONE($4, registered_at) >= $3)
            GROUP BY "trunc_registered_at!"
            ORDER BY "trunc_registered_at!""#,
            D::date_truncation(),
            path_id,
            start_datetime,
            state.posix_utc_offset_str,
        )
        .fetch_all(&state.pool)
        .await
        .ctx(Status::Internal)
        .log_msg("Failed to query chart data!")?;

        let now_date_part = D::from(now);

        let first_date_part = match start_datetime {
            Some(v) => D::from(v),
            None => match rows.first() {
                Some(row) => D::from(row.trunc_registered_at),
                // all-time without any rows.
                None => {
                    let datapoints = vec![Self {
                        x: now_date_part,
                        y: 0,
                    }];
                    return Ok(datapoints);
                }
            },
        };

        // Fill from the last row until now if the date part of the last row is not now.
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

            // Fill the gap until the row.
            if aggregator.next_date_part() < row_date_part {
                loop {
                    aggregator.push(0)?;

                    if aggregator.next_date_part() >= row_date_part {
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

#[derive(Serialize)]
pub struct StatsData {
    chart_data: ChartData,
    table_body: String,
}

impl StatsData {
    pub async fn build_response(
        chart_data: ChartData,
        state: &'static InnerAppState,
        path_id: Option<i64>,
        start_datetime: Option<PrimitiveDateTime>,
    ) -> Result<Json<Self>, RespErr> {
        let table_body = if let Some(path_id) = path_id {
            let referrer_counts =
                ReferrerCount::all_sorted_by_count(state, path_id, start_datetime).await?;
            let referrer_count_rows = CountRows::from(referrer_counts);

            ReferrersTableBody {
                referrer_count_rows,
            }
            .render()
        } else {
            let visits_counts = VisitCount::all_sorted_by_count(state, start_datetime).await?;
            let visit_count_rows = CountRows::from(visits_counts);

            VisitsTableBody { visit_count_rows }.render()
        }
        .ctx(Status::Internal)
        .log_msg("Failed to render the table body template!")?;

        Ok(Json(Self {
            chart_data,
            table_body,
        }))
    }
}
