use axum::{extract::State, Json};
use axum_ctx::RespErr;
use time::Duration;

use crate::{extractors::query_path::OptionalPathId, states::AppState};

use super::{ChartData, DataPoint, StartDatetime, StatsData, WholeDaysSinceFirstVisit};

pub async fn get(
    State(state): AppState,
    OptionalPathId(path_id): OptionalPathId,
) -> Result<Json<StatsData>, RespErr> {
    let start_datetime = StartDatetime::from_sub_duration(Duration::days(59));
    let start = start_datetime.start;

    let WholeDaysSinceFirstVisit {
        whole_days_since_first_visit,
        now,
        ..
    } = WholeDaysSinceFirstVisit::build(&state.pool, path_id, Some(start_datetime.clone())).await?;

    let (chart_data, start) = if whole_days_since_first_visit < 2 {
        let start = now - Duration::days(2);
        let start_datetime = StartDatetime { start, now };
        (
            ChartData::Hour(DataPoint::all(state, path_id, Some(start_datetime)).await?),
            start,
        )
    } else {
        (
            ChartData::Day(DataPoint::all(state, path_id, Some(start_datetime)).await?),
            start,
        )
    };

    StatsData::build_response(chart_data, &state.pool, path_id, Some(start)).await
}
