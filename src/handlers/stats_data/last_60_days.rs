use axum::{extract::State, Json};
use axum_ctx::{RespErr, RespErrCtx, RespErrExt, Status};
use time::{Duration, PrimitiveDateTime, Time};

use crate::{extractors::query_path::OptionalPathId, states::AppState};

use super::{ChartData, DataPoint, StatsData, WholeDaysSinceFirstVisit};

pub async fn get(
    State(state): AppState,
    OptionalPathId(path_id): OptionalPathId,
) -> Result<Json<StatsData>, RespErr> {
    let now = state.now_tz()?;

    let start_datetime = PrimitiveDateTime::new(now.date() - Duration::days(59), Time::MIDNIGHT);

    let WholeDaysSinceFirstVisit {
        whole_days_since_first_visit,
        ..
    } = WholeDaysSinceFirstVisit::build(state, path_id, now, Some(start_datetime)).await?;

    let (chart_data, start_datetime) = if whole_days_since_first_visit < 2 {
        let start_datetime = PrimitiveDateTime::new(
            now.date() - Duration::days(2),
            Time::from_hms(now.hour(), 0, 0)
                .ctx(Status::Internal)
                .log_msg("Failed to create Time for hour data!")?,
        );
        (
            ChartData::Hour(DataPoint::all(state, path_id, now, Some(start_datetime)).await?),
            start_datetime,
        )
    } else {
        (
            ChartData::Day(DataPoint::all(state, path_id, now, Some(start_datetime)).await?),
            start_datetime,
        )
    };

    StatsData::build_response(chart_data, state, path_id, Some(start_datetime)).await
}
