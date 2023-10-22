use axum::{
    extract::{Query, State},
    Json,
};
use axum_ctx::RespErr;
use time::Duration;

use crate::{extractors::query_path::QueryPath, states::AppState};

use super::{ChartData, DataPoint, StartDatetime};

pub async fn get(
    State(state): AppState,
    Query(path): Query<QueryPath>,
) -> Result<Json<ChartData>, RespErr> {
    let (_, path_id) = path.normalized_with_id(&state.pool).await?;

    let start_datetime = StartDatetime::from_sub_duration(Duration::days(2));

    let chart_data = ChartData::Hour(DataPoint::all(state, path_id, Some(start_datetime)).await?);

    Ok(Json(chart_data))
}
