use axum::{
    extract::{Query, State},
    Json,
};
use axum_ctx::RespErr;
use time::Duration;

use crate::{extractors::query_path::QueryPath, states::AppState};

use super::{
    contiguous_date_part::{ContiguousDay, ContiguousHour},
    DataPoint, DaysSinceFirstVisit, StartDatetime,
};

pub async fn get(
    State(state): AppState,
    Query(path): Query<QueryPath>,
) -> Result<Json<Vec<DataPoint>>, RespErr> {
    let (_, path_id) = path.normalized_with_id(&state.pool).await?;

    let start_datetime = StartDatetime::from_sub_duration(Duration::days(59));

    let DaysSinceFirstVisit {
        days_since_first_visit,
        ..
    } = DaysSinceFirstVisit::build(&state.pool, path_id, Some(start_datetime.clone())).await?;

    if days_since_first_visit <= 2 {
        DataPoint::all::<ContiguousHour>(state, path_id, Some(start_datetime)).await
    } else {
        DataPoint::all::<ContiguousDay>(state, path_id, Some(start_datetime)).await
    }
}
