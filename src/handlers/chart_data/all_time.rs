use axum::{
    extract::{Query, State},
    Json,
};
use axum_ctx::RespErr;

use crate::{extractors::query_path::QueryPath, states::AppState};

use super::{
    contiguous_date_part::{ContiguousDay, ContiguousHour, ContiguousMonth, ContiguousYear},
    DataPoint, DaysSinceFirstVisit,
};

pub async fn get(
    State(state): AppState,
    Query(path): Query<QueryPath>,
) -> Result<Json<Vec<DataPoint>>, RespErr> {
    let (_, path_id) = path.normalized_with_id(&state.pool).await?;

    let DaysSinceFirstVisit {
        days_since_first_visit,
        ..
    } = DaysSinceFirstVisit::build(&state.pool, path_id, None).await?;

    if days_since_first_visit <= 2 {
        DataPoint::all::<ContiguousHour>(&state.pool, path_id, None).await
    } else if days_since_first_visit <= 60 {
        DataPoint::all::<ContiguousDay>(&state.pool, path_id, None).await
    } else if days_since_first_visit <= 1826 {
        // Less than 5 years (about 60 months).
        DataPoint::all::<ContiguousMonth>(&state.pool, path_id, None).await
    } else {
        DataPoint::all::<ContiguousYear>(&state.pool, path_id, None).await
    }
}
