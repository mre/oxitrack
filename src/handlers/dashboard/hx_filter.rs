//! Active-search endpoints behind the dashboard's Pages / Referrers filter
//! inputs, following the htmx active-search pattern
//! (<https://htmx.org/examples/active-search/>): each keystroke issues a GET
//! whose response is just the filtered table rows, swapped into the table body.
use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Query, State};
use axum_ctx::RespResult;
use serde::Deserialize;

use crate::{
    handlers::{
        count_rows::CountRows,
        dashboard::page_stats::{self, PageStat},
        stats_data::{DateRange, referrer_count::ReferrerCount},
    },
    states::AppState,
};

/// `q` is the live search term typed into the filter input. Missing/empty means
/// "no filter", so the full list is returned.
#[derive(Deserialize, Default)]
pub struct SearchQuery {
    #[serde(default)]
    pub q: String,
}

impl SearchQuery {
    fn needle(&self) -> Option<String> {
        let trimmed = self.q.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_lowercase())
        }
    }
}

#[allow(clippy::cast_precision_loss)]
fn total_of<T: crate::handlers::count_rows::Count>(items: &[T]) -> f64 {
    items.iter().map(|i| i.count() as f64).sum()
}

#[derive(Template, WebTemplate)]
#[template(path = "pages_rows.html")]
pub struct PagesRows {
    pub pages: CountRows<PageStat>,
    pub range: DateRange,
}

pub async fn pages(
    State(state): AppState,
    Query(range): Query<DateRange>,
    Query(search): Query<SearchQuery>,
) -> RespResult<PagesRows> {
    let now = state.now_tz()?;
    let all = page_stats::all_sorted_by_count(state, &range, now).await?;

    // Denominator stays the full total so each page's `Share` keeps meaning
    // "share of all traffic" rather than "share of the filtered subset".
    let total = total_of(&all);

    let filtered: Vec<PageStat> = match search.needle() {
        Some(needle) => all
            .into_iter()
            .filter(|p| p.path.to_lowercase().contains(&needle))
            .collect(),
        None => all,
    };

    Ok(PagesRows {
        pages: CountRows::with_total(filtered, total),
        range,
    })
}

#[derive(Template, WebTemplate)]
#[template(path = "referrers_rows.html")]
pub struct ReferrersRows {
    pub referrers: CountRows<ReferrerCount>,
    pub range: DateRange,
}

pub async fn referrers(
    State(state): AppState,
    Query(range): Query<DateRange>,
    Query(search): Query<SearchQuery>,
) -> RespResult<ReferrersRows> {
    let all = ReferrerCount::all_sorted_by_count(
        state,
        None,
        range.start_datetime(),
        range.end_datetime(),
    )
    .await?;

    let total = total_of(&all);

    let filtered: Vec<ReferrerCount> = match search.needle() {
        Some(needle) => all
            .into_iter()
            .filter(|r| r.domain.to_lowercase().contains(&needle))
            .collect(),
        None => all,
    };

    Ok(ReferrersRows {
        referrers: CountRows::with_total(filtered, total),
        range,
    })
}
