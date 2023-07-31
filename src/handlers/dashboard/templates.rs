use crate::handlers::base_template::Base;
use askama::Template;

#[derive(Template)]
#[template(path = "stats.html")]
pub struct Stats<'a> {
    pub base: Base<'a>,
    pub svg: String,
    pub n_visits: usize,
    pub visits_per_day: f64,
    pub first_visit: String,
    pub last_visit: String,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index<'a> {
    pub base: Base<'a>,
    pub paths: Vec<String>,
}
