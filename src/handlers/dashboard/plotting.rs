use oxi_axum_helpers::{RespErr, RespErrCtx, Status};
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};
use tracing::error;

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

pub fn plot_history(
    history: &[i64],
    min: i64,
    max: i64,
    utc_offset: UtcOffset,
) -> Result<String, RespErr> {
    let mut svg = String::with_capacity(1024);

    {
        use plotters::prelude::*;

        let root = SVGBackend::with_string(&mut svg, (600, 600)).into_drawing_area();
        let mut chart = ChartBuilder::on(&root)
            .margin_left(14)
            .margin_right(14)
            .margin_top(8)
            .x_label_area_size(35)
            .y_label_area_size(35)
            .build_cartesian_2d(min..max, 1..history.len())
            .ctx(Status::Internal)?;

        chart
            .configure_mesh()
            .disable_x_mesh()
            .disable_y_mesh()
            .x_label_formatter(&|timestamp| x_label_formatter(*timestamp, utc_offset))
            .x_labels(4)
            .x_desc("Timestamp")
            .y_desc("Visits")
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
                    .map(|coord| Circle::new(coord, 2, BLUE.filled())),
            )
            .ctx(Status::Internal)?;

        root.present().ctx(Status::Internal)?;
    }

    Ok(svg)
}
