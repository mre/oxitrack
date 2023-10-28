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

    let WholeDaysSinceFirstVisit {
        whole_days_since_first_visit,
        now,
        ..
    } = WholeDaysSinceFirstVisit::build(&state.pool, path_id, None).await?;

    let chart_data = if whole_days_since_first_visit < 2 {
        let start_datetime = StartDatetime {
            start: now - Duration::days(2),
            now,
        };
        ChartData::Hour(DataPoint::all(state, path_id, Some(start_datetime)).await?)
    } else if whole_days_since_first_visit < 60 {
        let start_datetime = StartDatetime {
            start: now - Duration::days(59),
            now,
        };
        ChartData::Day(DataPoint::all(state, path_id, Some(start_datetime)).await?)
    } else if whole_days_since_first_visit < 1826 {
        // Less than 5 years (about 60 months).
        ChartData::Month(DataPoint::all(state, path_id, None).await?)
    } else {
        ChartData::Year(DataPoint::all(state, path_id, None).await?)
    };

    Ok(Json(chart_data))
}
