use axum::{Json, extract::State};
use axum_ctx::*;

use crate::{extractors::query_path::OptionalPathId, states::AppState};

use super::{
    ChartData, DataPoint, StatsData, WholeDaysSinceFirstVisit, day_data_start_datetime,
    hour_data_start_datetime,
};

pub async fn get(
    State(state): AppState,
    OptionalPathId(path_id): OptionalPathId,
) -> RespResult<Json<StatsData>> {
    let now = state.now_tz()?;

    let start_datetime = day_data_start_datetime(now);

    let hour_filter = WholeDaysSinceFirstVisit::build(state, path_id, now, Some(start_datetime))
        .await?
        .is_some_and(|v| v.whole_days_since_first_visit < 2);

    let (chart_data, start_datetime) = if hour_filter {
        let start_datetime = hour_data_start_datetime(now)?;

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
