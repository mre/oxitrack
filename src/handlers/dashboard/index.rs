use askama::Template;
use axum::{extract::State, response::Response};
use axum_ctx::RespErr;
use oxi_axum_helpers::TryIntoTemplResp;

use crate::{db::VisitCount, handlers::base_template::Base, states::AppState};

use super::count_rows::CountRows;

#[derive(Template)]
#[template(path = "index.html")]
struct Index<'a> {
    pub base: Base<'a>,
    pub base_url: &'static str,
    pub tracked_origin: &'static str,
    pub visit_count_rows: CountRows<VisitCount>,
}

pub async fn get(State(state): AppState) -> Result<Response, RespErr> {
    let counts = VisitCount::all_sorted_by_count(&state.pool).await?;
    let visit_count_rows = CountRows::from(counts);

    Index {
        base: Base::new("Dashboard"),
        base_url: state.base_url,
        tracked_origin: state.tracked_origin,
        visit_count_rows,
    }
    .try_into_resp()
}
