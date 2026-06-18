use askama::Template;
use askama_web::WebTemplate;
use axum::extract::{Query, State};
use axum_ctx::{RespErr, RespResult, StatusCode};
use serde::Deserialize;
use time::{Duration, OffsetDateTime};

use crate::{
    formatters::DateTimeVerboseFormatter,
    handlers::{
        base_template::Base,
        count_rows::CountRows,
        stats_data::{
            ChartBar, DateRange, PanelView, PresetButton, StatsLink, VisitFilter,
            WholeDaysSinceFirstVisit, build_chart, referrer_count::LinkedPage,
        },
    },
    states::{AppState, InnerAppState},
};

#[derive(Deserialize)]
pub struct QueryReferrer {
    pub domain: String,
}

/// Everything the referrer panel needs, shared by the full page and the htmx
/// partial so the two stay in lockstep.
pub struct ReferrerData {
    pub domain: String,
    pub total_visits: i64,
    pub per_day: f64,
    pub first_visit: Option<DateTimeVerboseFormatter>,
    pub pages: CountRows<LinkedPage>,
    pub chart: Vec<ChartBar>,
    pub range: DateRange,
    pub preset_buttons: Vec<PresetButton>,
}

impl ReferrerData {
    /// Builds the per-referrer ("reverse") view, or returns `None` if the
    /// domain was never recorded as a referrer.
    pub async fn build(
        state: &'static InnerAppState,
        domain: String,
        range: DateRange,
        now: OffsetDateTime,
    ) -> RespResult<Option<Self>> {
        let Some(referrer_id) = LinkedPage::id_for_domain(state, &domain).await? else {
            return Ok(None);
        };

        let filter = VisitFilter::referrer(referrer_id);

        let (pages_vec, chart, first) = tokio::try_join!(
            LinkedPage::all_for_referrer(
                state,
                referrer_id,
                range.start_datetime(),
                range.end_datetime(),
            ),
            build_chart(state, filter, &range, now),
            WholeDaysSinceFirstVisit::build(state, filter, now),
        )?;

        let total_visits: i64 = pages_vec.iter().map(|p| p.count).sum();

        let days = range.whole_days(now).or_else(|| {
            first
                .as_ref()
                .map(|f| f.whole_days_since_first_visit.max(1))
        });
        #[allow(clippy::cast_precision_loss)]
        let per_day = match days {
            Some(d) if d > 0 => total_visits as f64 / d as f64,
            _ => total_visits as f64,
        };

        let first_visit = first
            .map(|f| {
                state
                    .apply_utc_offset(f.first_visit)
                    .map(DateTimeVerboseFormatter)
            })
            .transpose()?;

        let pages = CountRows::from(pages_vec);

        let preset_buttons = StatsLink::new(&range, None)
            .with_referrer(Some(&domain))
            .preset_buttons("/hx/referrer", now.date());

        Ok(Some(Self {
            domain,
            total_visits,
            per_day,
            first_visit,
            pages,
            chart,
            range,
            preset_buttons,
        }))
    }
}

/// Applies the dashboard's "default to the last 90 days" rule.
pub fn default_range(range: DateRange, now: OffsetDateTime) -> DateRange {
    if range.from.is_none() && range.to.is_none() {
        let to = now.date();
        DateRange {
            from: Some(to - Duration::days(90)),
            to: Some(to),
        }
    } else {
        range
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "referrer.html")]
pub struct Referrer {
    pub base: Base<'static>,
    /// URL of the dashboard's Referrers tab, for the back breadcrumb.
    pub back_url: String,
    pub domain: String,
    pub total_visits: i64,
    pub per_day: f64,
    pub first_visit: Option<DateTimeVerboseFormatter>,
    pub pages: CountRows<LinkedPage>,
    pub chart: Vec<ChartBar>,
    pub range: DateRange,
    pub preset_buttons: Vec<PresetButton>,
}

pub async fn get(
    State(state): AppState,
    Query(q): Query<QueryReferrer>,
    Query(range): Query<DateRange>,
) -> RespResult<Referrer> {
    let now = state.now_tz()?;
    let range = default_range(range, now);

    let back_url = StatsLink::new(&range, None)
        .with_view(PanelView::Referrers)
        .url("/");

    let data = ReferrerData::build(state, q.domain, range, now)
        .await?
        .ok_or_else(|| {
            RespErr::new(StatusCode::NOT_FOUND).user_msg("That referrer has no recorded visits.")
        })?;

    Ok(Referrer {
        base: Base::new(state, "Referrer"),
        back_url,
        domain: data.domain,
        total_visits: data.total_visits,
        per_day: data.per_day,
        first_visit: data.first_visit,
        pages: data.pages,
        chart: data.chart,
        range: data.range,
        preset_buttons: data.preset_buttons,
    })
}
