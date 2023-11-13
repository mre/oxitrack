use axum::{extract::State, Json};
use axum_ctx::RespErr;
use time::Duration;

use crate::{extractors::query_path::OptionalPathId, states::AppState};

use super::{ChartData, DataPoint, StartDatetime};

pub async fn get(
    State(state): AppState,
    OptionalPathId(path_id): OptionalPathId,
) -> Result<Json<ChartData>, RespErr> {
    let start_datetime = StartDatetime::from_sub_duration(Duration::days(2));

    let chart_data = ChartData::Hour(DataPoint::all(state, path_id, Some(start_datetime)).await?);

    Ok(Json(chart_data))
}
