use rinja_axum::Template;

use crate::{db::VisitCount, handlers::count_rows::CountRows};

use super::referrer_count::ReferrerCount;

#[derive(Template)]
#[template(path = "visits_table_body.html")]
pub struct VisitsTableBody {
    pub base_url: &'static str,
    pub visit_count_rows: CountRows<VisitCount>,
}

#[derive(Template)]
#[template(path = "referrers_table_body.html")]
pub struct ReferrersTableBody {
    pub referrer_count_rows: CountRows<ReferrerCount>,
}
