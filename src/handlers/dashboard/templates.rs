use crate::handlers::base_template::Base;
use askama::Template;

#[derive(Template)]
#[template(path = "dashboard/plot.html")]
pub struct Plot<'a> {
    base: Base<'a>,
    svg: String,
}

#[derive(Template)]
#[template(path = "dashboard/index.html")]
pub struct Index<'a> {
    base: Base<'a>,
}
