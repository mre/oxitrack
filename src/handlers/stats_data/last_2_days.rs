use axum::{extract::State, Json};
use axum_ctx::*;

use crate::{extractors::query_path::OptionalPathId, states::AppState};

use super::{hour_data_start_datetime, ChartData, DataPoint, StatsData};

pub async fn get(
    State(state): AppState,
    OptionalPathId(path_id): OptionalPathId,
) -> RespResult<Json<StatsData>> {
    let now = state.now_tz()?;

    let start_datetime = hour_data_start_datetime(now)?;

    let chart_data =
        ChartData::Hour(DataPoint::all(state, path_id, now, Some(start_datetime)).await?);

    StatsData::build_response(chart_data, state, path_id, Some(start_datetime)).await
}
