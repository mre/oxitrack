use crate::handlers::base_template::Base;
use askama::Template;

#[derive(Template)]
#[template(path = "dashboard/stats.html")]
pub struct Stats<'a> {
    pub base: Base<'a>,
    pub svg: String,
}

#[derive(Template)]
#[template(path = "dashboard/index.html")]
pub struct Index<'a> {
    pub base: Base<'a>,
    pub paths: Vec<String>,
}
