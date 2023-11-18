use axum::{extract::State, Json};
use axum_ctx::{RespErr, RespErrCtx, RespErrExt, Status};
use time::{Duration, PrimitiveDateTime, Time};

use crate::{extractors::query_path::OptionalPathId, states::AppState};

use super::{ChartData, DataPoint, StatsData};

pub async fn get(
    State(state): AppState,
    OptionalPathId(path_id): OptionalPathId,
) -> Result<Json<StatsData>, RespErr> {
    let now = state.now_tz()?;

    let start_datetime = PrimitiveDateTime::new(
        now.date() - Duration::days(2),
        Time::from_hms(now.hour(), 0, 0)
            .ctx(Status::Internal)
            .log_msg("Failed to create Time for hour data!")?,
    );

    let chart_data =
        ChartData::Hour(DataPoint::all(state, path_id, now, Some(start_datetime)).await?);

    StatsData::build_response(chart_data, state, path_id, Some(start_datetime)).await
}
