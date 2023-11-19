use axum::{extract::State, Json};
use axum_ctx::{RespErr, Status};

use crate::{extractors::query_path::OptionalPathId, states::AppState};

use super::{
    day_data_start_datetime, hour_data_start_datetime, ChartData, DataPoint, StatsData,
    WholeDaysSinceFirstVisit,
};

pub async fn get(
    State(state): AppState,
    OptionalPathId(path_id): OptionalPathId,
) -> Result<Json<StatsData>, RespErr> {
    let now = state.now_tz()?;

    let Some(WholeDaysSinceFirstVisit {
        whole_days_since_first_visit,
        ..
    }) = WholeDaysSinceFirstVisit::build(state, path_id, now, None).await?
    else {
        return Err(RespErr::new(Status::NotFound)
            .user_msg("The requested path has no counted visits yet."));
    };

    let (chart_data, start_datetime) = if whole_days_since_first_visit < 2 {
        let start_datetime = hour_data_start_datetime(now)?;

        (
            ChartData::Hour(DataPoint::all(state, path_id, now, Some(start_datetime)).await?),
            Some(start_datetime),
        )
    } else if whole_days_since_first_visit < 60 {
        let start_datetime = day_data_start_datetime(now);

        (
            ChartData::Day(DataPoint::all(state, path_id, now, Some(start_datetime)).await?),
            Some(start_datetime),
        )
    } else if whole_days_since_first_visit < 1461 {
        // Less than 4 years (48 months).
        (
            ChartData::Month(DataPoint::all(state, path_id, now, None).await?),
            None,
        )
    } else {
        (
            ChartData::Year(DataPoint::all(state, path_id, now, None).await?),
            None,
        )
    };

    StatsData::build_response(chart_data, state, path_id, start_datetime).await
}
