use axum::{
    extract::{Query, State},
    response::Response,
};
use futures::TryStreamExt;
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status, TryIntoTemplResp};
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};

use crate::{
    db::{Id, TimeStamp},
    handlers::{base_template::Base, queries::PathQuery, AppStateT},
};

use super::{plotting, templates};

fn formatted_datetime_from_timestamp(
    timestamp: i64,
    utc_offset: UtcOffset,
) -> Result<String, RespErr> {
    OffsetDateTime::from_unix_timestamp(timestamp)
        .ctx(Status::Internal)
        .err_msg("Failed to parse datetime from unix timestamp!")?
        .to_offset(utc_offset)
        .format(&Rfc3339)
        .ctx(Status::Internal)
        .err_msg("Failed to format datetime!")
}

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

    let history = sqlx::query_as!(
        TimeStamp,
        "SELECT timestamp FROM visits
        WHERE path_id = $1
        ORDER BY timestamp",
        path_id,
    )
    .fetch(&*state.db)
    .map_ok(|row| row.timestamp.unix_timestamp())
    .try_collect::<Vec<_>>()
    .await
    .ctx(Status::Internal)
    .err_msg_lz(|| format!("History query failed for path {path}!"))?;

    let n_visits = history.len();

    let first_visit = *history
        .first()
        .ctx(Status::NotFound)
        .user_msg_lz(|| format!("The requested path {path} has no counted visits yet."))?;

    let last_visit = *history
        .last()
        .ctx(Status::Internal)
        .err_msg("Last item does not exist although the first one exists!")?;

    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let secs_per_day = 86_400;
    let days_since_first_visit = 1 + (now - first_visit) / secs_per_day;
    let visits_per_day = n_visits as f64 / days_since_first_visit as f64;

    let svg = plotting::plot_history(&history, first_visit, last_visit, state.utc_offset)
        .err_msg_lz(|| format!("Failed to plot the call history for path {path}!"))?;

    templates::Stats {
        base: Base::new(path),
        tracked_origin: &state.tracked_origin,
        path,
        svg,
        n_visits,
        visits_per_day,
        first_visit: formatted_datetime_from_timestamp(first_visit, state.utc_offset)?,
        last_visit: formatted_datetime_from_timestamp(last_visit, state.utc_offset)?,
    }
    .try_into_resp()
}
