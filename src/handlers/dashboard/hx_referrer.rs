use askama::Template;
use askama_web::WebTemplate;
use axum::{
    extract::{Query, State},
    http::{HeaderName, HeaderValue},
    response::IntoResponse,
};
use axum_ctx::{RespErr, RespErrCtx, RespErrExt, RespResult, StatusCode};

use crate::{
    handlers::{
        count_rows::CountRows,
        dashboard::referrer::{QueryReferrer, ReferrerData},
        stats_data::{ChartBar, DateRange, PresetButton, StatsLink, referrer_count::LinkedPage},
    },
    states::AppState,
};

const HX_PUSH_URL: HeaderName = HeaderName::from_static("hx-push-url");

#[derive(Template, WebTemplate)]
#[template(path = "referrer_panel.html")]
pub struct HxReferrer {
    pub total_visits: i64,
    pub pages: CountRows<LinkedPage>,
    pub chart: Vec<ChartBar>,
    pub range: DateRange,
    pub preset_buttons: Vec<PresetButton>,
}

pub async fn get(
    State(state): AppState,
    Query(q): Query<QueryReferrer>,
    Query(range): Query<DateRange>,
) -> RespResult<impl IntoResponse> {
    let now = state.now_tz()?;
    let range = range.or_last_90_days(now);

    // Address bar after the swap: the canonical full-page referrer URL.
    let push_url = StatsLink::new(&range, None)
        .with_referrer(Some(&q.domain))
        .url("/referrer");

    let data = ReferrerData::build(state, q.domain, range, now)
        .await?
        .ok_or_else(|| {
            RespErr::new(StatusCode::NOT_FOUND).user_msg("That referrer has no recorded visits.")
        })?;

    let body = HxReferrer {
        total_visits: data.total_visits,
        pages: data.pages,
        chart: data.chart,
        range: data.range,
        preset_buttons: data.preset_buttons,
    };

    let push_header = HeaderValue::try_from(push_url)
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Failed to build HX-Push-Url header")?;

    Ok(([(HX_PUSH_URL, push_header)], body))
}
