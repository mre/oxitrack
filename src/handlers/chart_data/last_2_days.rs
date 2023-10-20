use axum::{
    extract::{Query, State},
    Json,
};
use axum_ctx::RespErr;
use time::{Duration, OffsetDateTime};

use crate::{extractors::query_path::QueryPath, states::AppState};

use super::{contiguous_date_part::ContiguousHour, DataPoint};

pub async fn get(
    State(state): AppState,
    Query(path): Query<QueryPath>,
) -> Result<Json<Vec<DataPoint>>, RespErr> {
    let (_, path_id) = path.normalized_with_id(&state.pool).await?;

    let now = OffsetDateTime::now_utc();
    let start_date = now - Duration::days(2);

    DataPoint::all::<ContiguousHour>(&state.pool, path_id, Some(start_date)).await
}
