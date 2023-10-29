use axum::{
    extract::{Query, State},
    Json,
};
use axum_ctx::RespErr;
use time::Duration;

use crate::{extractors::query_path::QueryPath, states::AppState};

use super::{ChartData, DataPoint, StartDatetime, WholeDaysSinceFirstVisit};

pub async fn get(
    State(state): AppState,
    Query(path): Query<QueryPath>,
) -> Result<Json<ChartData>, RespErr> {
    let (_, path_id) = path.normalized_with_id(&state.pool).await?;

    let start_datetime = StartDatetime::from_sub_duration(Duration::days(59));

    let WholeDaysSinceFirstVisit {
        whole_days_since_first_visit,
        now,
        ..
    } = WholeDaysSinceFirstVisit::build(&state.pool, path_id, Some(start_datetime.clone())).await?;

    let chart_data = if whole_days_since_first_visit < 2 {
        let start_datetime = StartDatetime {
            start: now - Duration::days(2),
            now,
        };
        ChartData::Hour(DataPoint::all(state, Some(path_id), Some(start_datetime)).await?)
    } else {
        ChartData::Day(DataPoint::all(state, Some(path_id), Some(start_datetime)).await?)
    };

    Ok(Json(chart_data))
}
