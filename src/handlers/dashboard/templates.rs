use crate::handlers::base_template::Base;
use askama::Template;

use super::index::CountsRows;

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index<'a> {
    pub base: Base<'a>,
    pub tracked_origin: &'a str,
    pub counts_rows: CountsRows,
}

#[derive(Template)]
#[template(path = "stats.html")]
pub struct Stats<'a> {
    pub base: Base<'a>,
    pub tracked_origin: &'a str,
    pub path: &'a str,
    pub history: String,
    pub min_chart_timestamp: i64,
    pub max_chart_timestamp: i64,
    pub n_visits: usize,
    pub visits_per_day: f64,
}
