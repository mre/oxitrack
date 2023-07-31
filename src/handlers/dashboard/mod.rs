mod templates;

use axum::{
    extract::{Query, State},
    response::Response,
};
use futures::TryStreamExt;
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status, TryIntoTemplResp};
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};
use tracing::error;

use crate::db::{self, Id, TimeStamp};

use self::templates::Index;

use super::{base_template::Base, queries::PathQuery, AppStateT};

pub async fn index(State(state): AppStateT) -> Result<Response, RespErr> {
    let paths = sqlx::query_as!(db::Path, "SELECT path FROM paths ORDER BY path")
        .map(|row| row.path)
        .fetch_all(&*state.db)
        .await
        .ctx(Status::Internal)
        .err_msg("Paths query failed!")?;

    Index {
        base: Base { title: "Dashboard" },
        paths,
    }
    .try_into_resp()
}

fn x_label_formatter(timestamp: i64, utc_offset: UtcOffset) -> String {
    match OffsetDateTime::from_unix_timestamp(timestamp) {
        Ok(timestamp) => timestamp
            .to_offset(utc_offset)
            .format(&Rfc3339)
            .unwrap_or_else(|e| {
                error!("Failed to format datetime for x labels!\n{e:?}");

                String::new()
            }),
        Err(e) => {
            error!("Failed to parse datetime from unix timestamp for x labels!\n{e:?}");

            String::new()
        }
    }
}

fn plot_history(
    history: Vec<i64>,
    min: i64,
    max: i64,
    utc_offset: UtcOffset,
) -> Result<String, RespErr> {
    let mut svg = String::with_capacity(1024);

    {
        use plotters::prelude::*;

        let root = SVGBackend::with_string(&mut svg, (900, 650)).into_drawing_area();
        let mut chart = ChartBuilder::on(&root)
            .margin_left(14)
            .margin_right(14)
            .margin_top(8)
            .x_label_area_size(32)
            .y_label_area_size(32)
            .build_cartesian_2d(min..max, 1..history.len())
            .ctx(Status::Internal)?;

        chart
            .configure_mesh()
            .disable_x_mesh()
            .disable_y_mesh()
            .x_label_formatter(&|timestamp| x_label_formatter(*timestamp, utc_offset))
            .x_labels(4)
            .x_desc("Timestamp")
            .y_desc("Calls")
            .draw()
            .ctx(Status::Internal)?;

        chart
            .draw_series(LineSeries::new(
                history.iter().copied().zip(1..=history.len()),
                BLACK,
            ))
            .ctx(Status::Internal)?;

        chart
            .draw_series(
                history
                    .iter()
                    .copied()
                    .zip(1..=history.len())
                    .map(|coord| Circle::new(coord, 3, BLUE.filled())),
            )
            .ctx(Status::Internal)?;

        root.present().ctx(Status::Internal)?;
    }

    Ok(svg)
}

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

pub async fn stats(
    State(state): AppStateT,
    Query(path): Query<PathQuery>,
) -> Result<Response, RespErr> {
    let path = path.normalized();

    let path_id = sqlx::query_as!(Id, "SELECT id FROM paths WHERE path = $1", path)
        .fetch_one(&*state.db)
        .await
        .ctx(Status::NotFound)
        .err_msg_lz(|| format!("Path {path} not found!"))?
        .id;

    let history = sqlx::query_as!(
        TimeStamp,
        "SELECT timestamp FROM calls WHERE path_id = $1 ORDER BY timestamp",
        path_id,
    )
    .fetch(&*state.db)
    .map_ok(|row| row.timestamp.unix_timestamp())
    .try_collect::<Vec<_>>()
    .await
    .ctx(Status::Internal)
    .err_msg_lz(|| format!("History query failed for path {path}!"))?;

    let n_visits = history.len();

    let first_visit = *history.first().ctx(Status::NotFound).user_msg_lz(|| {
        format!("The requested path {path} does not have any counted visit yet.")
    })?;

    let last_visit = *history
        .last()
        .ctx(Status::Internal)
        .err_msg("Last item does not exist although the first one exists!")?;

    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    let secs_per_day = 86_400;
    let days_since_first_visit = 1 + (now - first_visit) / secs_per_day;
    let visits_per_day = n_visits as f64 / days_since_first_visit as f64;

    let svg = plot_history(history, first_visit, last_visit, state.utc_offset)
        .err_msg_lz(|| format!("Failed to plot the call history for path {path}!"))?;

    templates::Stats {
        base: Base { title: path },
        svg,
        n_visits,
        visits_per_day,
        first_visit: formatted_datetime_from_timestamp(first_visit, state.utc_offset)?,
        last_visit: formatted_datetime_from_timestamp(last_visit, state.utc_offset)?,
    }
    .try_into_resp()
}
