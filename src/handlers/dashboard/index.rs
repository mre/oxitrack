use askama::Template;
use axum::{extract::State, response::Response};
use axum_ctx::RespErr;
use oxi_axum_helpers::TryIntoTemplResp;
use std::slice::Iter;

use crate::{db::VisitCount, handlers::base_template::Base, states::AppState};

pub struct CountsRows {
    counts: Vec<VisitCount>,
    mult_factor: f64,
}

pub struct CountsRowsIter<'a> {
    counts_iter: Iter<'a, VisitCount>,
    mult_factor: f64,
}

impl<'a> Iterator for CountsRowsIter<'a> {
    type Item = (&'a VisitCount, f64);

    fn next(&mut self) -> Option<Self::Item> {
        self.counts_iter
            .next()
            .map(|path_count| (path_count, path_count.count as f64 * self.mult_factor))
    }
}

impl<'a> IntoIterator for &'a CountsRows {
    type IntoIter = CountsRowsIter<'a>;
    type Item = <Self::IntoIter as Iterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        CountsRowsIter {
            counts_iter: self.counts.iter(),
            mult_factor: self.mult_factor,
        }
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct Index<'a> {
    pub base: Base<'a>,
    pub tracked_origin: &'static str,
    pub counts_rows: CountsRows,
}

pub async fn get(State(state): AppState) -> Result<Response, RespErr> {
    let counts = VisitCount::all_sorted(&state.pool).await?;
    let total_n_visits = counts.iter().map(|c| c.count).sum::<i64>();
    let mult_factor = 100.0 / total_n_visits as f64;
    let counts_rows = CountsRows {
        counts,
        mult_factor,
    };

    Index {
        base: Base::new("Dashboard"),
        tracked_origin: state.tracked_origin,
        counts_rows,
    }
    .try_into_resp()
}
