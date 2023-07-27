mod templates;

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::Response,
};
use futures::TryStreamExt;
use oxi_axum_helpers::{RespErr, RespErrCtx, RespErrExt, Status, TryIntoTemplResp};
use time::{format_description::well_known::Rfc3339, UtcOffset};
use tracing::instrument;

use crate::db::{self, Id, TimeStamp};

use self::templates::Index;

use super::{base_template::Base, states::AppState, AppStateT};

#[instrument(skip_all)]
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

fn plot_history(svg: &mut String, history: Vec<i64>, utc_offset: UtcOffset) -> Result<(), RespErr> {
    use plotters::prelude::*;

    let min = *history
        .first()
        .ctx(Status::Internal)
        .err_msg("Empty history of an existing path!")?;

    let max = *history.last().ctx(Status::Internal)?;

    let root = SVGBackend::with_string(svg, (1280, 700)).into_drawing_area();
    let mut chart = ChartBuilder::on(&root)
        .margin_left(12)
        .margin_right(12)
        .margin_top(8)
        .x_label_area_size(32)
        .y_label_area_size(32)
        .build_cartesian_2d(min..max, 1..history.len())
        .ctx(Status::Internal)?;

    chart
        .configure_mesh()
        .disable_x_mesh()
        .disable_y_mesh()
        .x_label_formatter(&|timestamp| {
            time::OffsetDateTime::from_unix_timestamp(*timestamp)
                .unwrap()
                .to_offset(utc_offset)
                .format(&Rfc3339)
                .unwrap()
        })
        .x_labels(5)
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

    Ok(())
}

async fn handle_plot(state: Arc<AppState>, path: &str) -> Result<Response, RespErr> {
    let path_id = sqlx::query_as!(Id, "SELECT id FROM paths WHERE path = $1", path)
        .fetch_one(&*state.db)
        .await
        .ctx(Status::NotFound)
        .err_msg("Path not found!")?
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
    .err_msg("History query failed!")?;

    let mut svg = String::with_capacity(1024);
    plot_history(&mut svg, history, state.utc_offset)?;

    templates::Plot {
        base: Base { title: path },
        svg,
    }
    .try_into_resp()
}

#[instrument(skip_all)]
pub async fn plot_index(State(state): AppStateT) -> Result<Response, RespErr> {
    let path = "";

    handle_plot(state, path).await
}

#[instrument(skip_all)]
pub async fn plot(State(state): AppStateT, Path(path): Path<String>) -> Result<Response, RespErr> {
    let path = path.trim_end_matches('/');

    handle_plot(state, path).await
}
