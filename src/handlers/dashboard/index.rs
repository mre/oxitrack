use axum::{extract::State, response::Response};
use oxi_axum_helpers::{RespErr, TryIntoTemplResp};
use std::slice::Iter;

use crate::{
    db::Count,
    handlers::{base_template::Base, AppStateT},
};

use super::templates::Index;

pub struct CountsRows {
    counts: Vec<Count>,
    mult_factor: f64,
}

pub struct CountsRowsIter<'a> {
    counts_iter: Iter<'a, Count>,
    mult_factor: f64,
}

impl<'a> Iterator for CountsRowsIter<'a> {
    type Item = (&'a Count, f64);

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

pub async fn get(State(state): AppStateT) -> Result<Response, RespErr> {
    let counts = Count::query_all_sorted(&state.db).await?;
    let total_n_visits = counts.iter().map(|c| c.count).sum::<i64>();
    let mult_factor = 100.0 / total_n_visits as f64;
    let counts_rows = CountsRows {
        counts,
        mult_factor,
    };

    Index {
        base: Base::new("Dashboard"),
        tracked_origin: &state.tracked_origin,
        counts_rows,
    }
    .try_into_resp()
}
