use askama::Template;
use axum::{
    extract::{Path, State},
    response::Html,
};
use axum_ctx::{RespErr, Status};
use oxi_axum_helpers::TryIntoTemplResp;
use time::{Duration, OffsetDateTime};

use crate::{db::VisitCount, handlers::count_rows::CountRows, states::AppState};

#[derive(Template)]
#[template(path = "visits_table_body.html")]
pub struct VisitsTableBody {
    pub visit_count_rows: CountRows<VisitCount>,
}

pub async fn get(
    State(state): AppState,
    Path(filter): Path<String>,
) -> Result<Html<String>, RespErr> {
    let start_datetime = match filter.as_str() {
        "all-time" => None,
        "last-60-days" => Some(OffsetDateTime::now_utc() - Duration::days(59)),
        "last-2-days" => Some(OffsetDateTime::now_utc() - Duration::days(2)),
        _ => return Err(RespErr::new(Status::BadRequest).user_msg("Wrong start datetime filter!")),
    };

    let counts = VisitCount::all_sorted_by_count(&state.pool, start_datetime).await?;
    let visit_count_rows = CountRows::from(counts);

    VisitsTableBody { visit_count_rows }.try_into_resp()
}
