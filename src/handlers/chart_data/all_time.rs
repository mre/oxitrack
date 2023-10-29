use axum::{extract::State, Json};
use axum_ctx::RespErr;
use time::Duration;

use crate::{extractors::query_path::OptionalPathId, states::AppState};

use super::{ChartData, DataPoint, StartDatetime, WholeDaysSinceFirstVisit};

pub async fn get(
    State(state): AppState,
    OptionalPathId(path_id): OptionalPathId,
) -> Result<Json<ChartData>, RespErr> {
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
    } else if whole_days_since_first_visit < 1461 {
        // Less than 4 years (48 months).
        ChartData::Month(DataPoint::all(state, path_id, None).await?)
    } else {
        ChartData::Year(DataPoint::all(state, path_id, None).await?)
    };

    Ok(Json(chart_data))
}
