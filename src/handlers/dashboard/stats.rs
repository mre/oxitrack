use axum::{
    extract::{Query, State},
    response::Response,
};
use futures::TryStreamExt;
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status, TryIntoTemplResp};

use crate::{
    db::{Id, TimeStamp},
    handlers::{base_template::Base, queries::PathQuery, AppStateT},
};

use super::templates;

// Milliseconds per day.
const MS_PER_DAY: i64 = 86_400_000;
const MS_PER_DAY_F: f64 = MS_PER_DAY as f64;

pub async fn get(
    State(state): AppStateT,
    Query(path): Query<PathQuery>,
) -> Result<Response, RespErr> {
    let path = path.normalized();

    let path_id = sqlx::query_as!(
        Id,
        "SELECT id FROM paths
        WHERE path = $1",
        path
    )
    .fetch_one(&*state.db)
    .await
    .ctx(Status::NotFound)
    .err_msg_lz(|| format!("Path {path} not found!"))?
    .id;

    // Converted to ms timestamp.
    let history = sqlx::query_as!(
        TimeStamp,
        "SELECT timestamp FROM visits
        WHERE path_id = $1
        ORDER BY timestamp",
        path_id,
    )
    .fetch(&*state.db)
    .map_ok(|row| 1000 * row.timestamp.unix_timestamp())
    .try_collect::<Vec<_>>()
    .await
    .ctx(Status::Internal)
    .err_msg_lz(|| format!("History query failed for path {path}!"))?;

    let n_visits = history.len();

    let min_timestamp = *history
        .first()
        .ctx(Status::NotFound)
        .user_msg_lz(|| format!("The requested path {path} has no counted visits yet."))?;

    let now_ms = 1000 * time::OffsetDateTime::now_utc().unix_timestamp();
    let ms_since_first_visit = now_ms - min_timestamp;
    let visits_per_day = if ms_since_first_visit > MS_PER_DAY {
        n_visits as f64 * MS_PER_DAY_F / ms_since_first_visit as f64
    } else {
        n_visits as f64
    };

    let x_axis_padding = (ms_since_first_visit as f64 * 0.004) as i64;
    let min_chart_timestamp = min_timestamp - x_axis_padding;
    let max_chart_timestamp = now_ms + x_axis_padding;

    let history = serde_json::to_string(&history)
        .ctx(Status::Internal)
        .err_msg("Failed to convert history to JSON string!")?;

    templates::Stats {
        base: Base::new(path),
        tracked_origin: &state.tracked_origin,
        path,
        history,
        min_chart_timestamp,
        max_chart_timestamp,
        n_visits,
        visits_per_day,
    }
    .try_into_resp()
}
