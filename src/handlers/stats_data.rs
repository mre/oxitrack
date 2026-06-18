mod chart_data_aggregator;
mod contiguous_date_part;
pub mod referrer_count;
pub mod whole_days_since_first_visit;

pub use whole_days_since_first_visit::WholeDaysSinceFirstVisit;

use axum_ctx::{RespErrCtx, RespErrExt, RespResult, StatusCode};
use serde::{Deserialize, Deserializer, Serialize};
use time::macros::format_description;
use time::{Date, Duration, OffsetDateTime, PrimitiveDateTime, Time};

use crate::{db::Db, states::InnerAppState};

use chart_data_aggregator::ChartDataAggregator;
use contiguous_date_part::{
    ContiguousDatePart, ContiguousDay, ContiguousHour, ContiguousMonth, ContiguousYear,
};

#[derive(sqlx::FromRow)]
struct TruncDateCount {
    trunc_registered_at: PrimitiveDateTime,
    count: i64,
}

/// A single bar in the chart.
pub struct ChartBar {
    pub label: String,
    pub count: u64,
}

/// Optional dimension filters shared by the chart and aggregate queries.
///
/// A `None` field means "don't filter on this dimension", so
/// `VisitFilter::default()` matches every visit. Threading one struct instead
/// of a growing list of `Option<i64>` parameters keeps call sites unambiguous
/// as new drill-down views (per-path, per-referrer, …) are added.
#[derive(Clone, Copy, Default)]
pub struct VisitFilter {
    pub path_id: Option<i64>,
    pub referrer_id: Option<i64>,
}

impl VisitFilter {
    /// Filter to a single path.
    pub const fn path(path_id: i64) -> Self {
        Self {
            path_id: Some(path_id),
            referrer_id: None,
        }
    }

    /// Filter to a single referrer (the "reverse" view).
    pub const fn referrer(referrer_id: i64) -> Self {
        Self {
            path_id: None,
            referrer_id: Some(referrer_id),
        }
    }

    /// Filter to an optional path; `None` matches every path.
    pub const fn from_path_opt(path_id: Option<i64>) -> Self {
        Self {
            path_id,
            referrer_id: None,
        }
    }
}

/// Which table the dashboard's tab strip is currently showing. Parsed leniently
/// from the `view` query parameter so a stray value never 400s — it just falls
/// back to the default [`Pages`](Self::Pages) view.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelView {
    #[default]
    Pages,
    Referrers,
}

impl PanelView {
    pub fn from_opt(raw: Option<&str>) -> Self {
        match raw {
            Some("referrers") => Self::Referrers,
            _ => Self::Pages,
        }
    }

    /// Stable key used in URLs (`?view=...`). The default `Pages` view returns
    /// `None` so its URLs stay clean (no redundant `view=pages`).
    pub const fn url_value(self) -> Option<&'static str> {
        match self {
            Self::Pages => None,
            Self::Referrers => Some("referrers"),
        }
    }

    pub const fn is_referrers(self) -> bool {
        matches!(self, Self::Referrers)
    }
}

/// Query extractor for the dashboard tab. Lenient: any unrecognized `view`
/// value falls back to [`PanelView::Pages`] via [`PanelView::from_opt`].
#[derive(Deserialize, Default)]
pub struct ViewQuery {
    pub view: Option<String>,
}

impl ViewQuery {
    pub fn panel(&self) -> PanelView {
        PanelView::from_opt(self.view.as_deref())
    }
}

/// Deserialize an `Option<Date>` from the ISO 8601 representation used by the
/// dashboard's `from`/`to` query parameters.
///
/// Missing keys, empty strings, and unparseable values all map to `None`, so a
/// stray or invalid `from=`/`to=` never breaks the page. Serialization uses
/// `Date`'s built-in `Serialize` impl (enabled by `time`'s `serde-human-readable`
/// feature), which already produces the `YYYY-MM-DD` form.
fn deserialize_opt_date<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Date>, D::Error> {
    let opt = Option::<String>::deserialize(d)?;
    let fmt = format_description!("[year]-[month]-[day]");
    Ok(opt
        .filter(|s| !s.is_empty())
        .and_then(|s| Date::parse(&s, fmt).ok()))
}

/// Arbitrary date range filter for chart/stats queries.
///
/// Implements `Serialize`/`Deserialize` so it can be used directly as an axum
/// `Query` extractor and round-tripped back to a URL query string via
/// [`Self::query_string`]. Adding more filter parameters in the future is just a
/// matter of extending this struct.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct DateRange {
    #[serde(
        default,
        deserialize_with = "deserialize_opt_date",
        skip_serializing_if = "Option::is_none"
    )]
    pub from: Option<Date>,
    #[serde(
        default,
        deserialize_with = "deserialize_opt_date",
        skip_serializing_if = "Option::is_none"
    )]
    pub to: Option<Date>,
}

impl DateRange {
    /// The dashboard's default window: when neither bound is set, fall back to
    /// the last 90 days (anchored at `now`). An explicit range is returned
    /// unchanged. Shared by every dashboard entry point so the default stays
    /// consistent.
    #[must_use]
    pub fn or_last_90_days(self, now: OffsetDateTime) -> Self {
        if self.from.is_none() && self.to.is_none() {
            let to = now.date();
            Self {
                from: Some(to - Duration::days(90)),
                to: Some(to),
            }
        } else {
            self
        }
    }

    pub fn start_datetime(&self) -> Option<PrimitiveDateTime> {
        self.from.map(|d| PrimitiveDateTime::new(d, Time::MIDNIGHT))
    }

    pub fn end_datetime(&self) -> Option<PrimitiveDateTime> {
        self.to
            .map(|d| PrimitiveDateTime::new(d + Duration::days(1), Time::MIDNIGHT))
    }

    pub fn whole_days(&self, now: OffsetDateTime) -> Option<i64> {
        let from = self.from?;
        let to = self.to.unwrap_or_else(|| now.date());
        Some((to - from).whole_days().max(1))
    }

    pub fn label(&self) -> String {
        // Friendly label for common presets.
        if let Some(p) = self.matched_preset() {
            return p.label().to_string();
        }

        let fmt = format_description!("[year]-[month]-[day]");
        match (self.from, self.to) {
            (None, _) => "All time".to_string(),
            (Some(f), None) => format!("Since {}", f.format(fmt).unwrap_or_default()),
            (Some(f), Some(t)) => format!(
                "{} – {}",
                f.format(fmt).unwrap_or_default(),
                t.format(fmt).unwrap_or_default()
            ),
        }
    }

    /// Returns the [`Preset`] this range corresponds to, if any.
    ///
    /// Used by templates to render the active filter button server-side without
    /// any client-side logic.
    pub fn matched_preset(&self) -> Option<Preset> {
        Preset::ALL
            .iter()
            .copied()
            .find(|p| p.matches(self.from, self.to))
    }

    /// Returns the range as URL query parameters (without leading `?` or `&`).
    ///
    /// Serialization goes through `serde_urlencoded`, so adding new fields to
    /// `DateRange` (or any future combined query type) automatically extends the
    /// produced query string without requiring manual concatenation here.
    /// Only fields that are `Some` are included; an empty string is returned if
    /// no fields are set.
    pub fn query_string(&self) -> String {
        serde_urlencoded::to_string(self).unwrap_or_default()
    }

    /// Returns the range as URL query parameters with a leading `&`,
    /// suitable for appending to an existing query string. Empty if no params are set.
    pub fn query_suffix(&self) -> String {
        let qs = self.query_string();
        if qs.is_empty() {
            String::new()
        } else {
            format!("&{qs}")
        }
    }
}

/// A built-in date-range filter preset, rendered as one of the buttons in the
/// dashboard's filter bar.
///
/// Defining the presets server-side keeps the templates and the JS-free htmx
/// flow in sync: the server is the single source of truth for which presets
/// exist, what their labels are, and which one is currently active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    Today,
    Last7,
    Last30,
    Last90,
    Last365,
    AllTime,
}

impl Preset {
    /// All presets in display order.
    pub const ALL: [Self; 6] = [
        Self::Today,
        Self::Last7,
        Self::Last30,
        Self::Last90,
        Self::Last365,
        Self::AllTime,
    ];

    /// Stable identifier used as the `data-preset` attribute and (optionally)
    /// in URLs. Not currently parsed back, but useful for tests / debugging.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::Last7 => "7",
            Self::Last30 => "30",
            Self::Last90 => "90",
            Self::Last365 => "365",
            Self::AllTime => "all",
        }
    }

    /// Human-readable button label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Today => "Today",
            Self::Last7 => "Last 7 days",
            Self::Last30 => "Last 30 days",
            Self::Last90 => "Last 90 days",
            Self::Last365 => "Last year",
            Self::AllTime => "All time",
        }
    }

    /// Number of days back from `today` this preset covers, or `None` for
    /// "all time" (which translates to no `from`/`to`).
    const fn days_back(self) -> Option<i64> {
        match self {
            Self::Today => Some(0),
            Self::Last7 => Some(7),
            Self::Last30 => Some(30),
            Self::Last90 => Some(90),
            Self::Last365 => Some(365),
            Self::AllTime => None,
        }
    }

    /// Builds the concrete [`DateRange`] this preset represents, given today.
    pub fn date_range(self, today: Date) -> DateRange {
        self.days_back()
            .map_or_else(DateRange::default, |days| DateRange {
                from: Some(today - Duration::days(days)),
                to: Some(today),
            })
    }

    /// Returns `true` if `(from, to)` matches this preset's shape.
    ///
    /// "All time" matches when both bounds are unset; the others match when
    /// `to - from` equals the preset's day-span (the absolute date doesn't
    /// matter, so cached buttons still light up correctly across midnight).
    fn matches(self, from: Option<Date>, to: Option<Date>) -> bool {
        match (self.days_back(), from, to) {
            (None, None, None) => true,
            (Some(days), Some(f), Some(t)) => (t - f).whole_days() == days,
            _ => false,
        }
    }
}

/// A preset rendered as one filter button, fully prepared by the server so
/// the template can stay logic-free and the frontend needs no JS to wire it up.
///
/// `hx_url` is the URL the button hits via `hx-get`; `active` controls the
/// CSS state. Handlers build these (one per [`Preset`]) and pass them to the
/// template.
pub struct PresetButton {
    pub key: &'static str,
    pub label: &'static str,
    pub hx_url: String,
    pub active: bool,
}

/// Serializable view of the dashboard's query parameters, used by handlers
/// to build URLs for filter buttons, the live indicator, and the
/// `HX-Push-Url` response header.
///
/// Flattening [`DateRange`] keeps `from`/`to` at the top level while letting
/// `serde_urlencoded` handle all encoding/skipping concerns. Adding more
/// filter parameters in the future just means adding fields here, and every
/// URL the dashboard generates will pick them up automatically.
#[derive(Serialize)]
pub struct StatsLink<'a> {
    #[serde(flatten)]
    pub range: &'a DateRange,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<&'a str>,
    /// Serialized as `domain=...` to match the `/referrer` route's parameter.
    #[serde(rename = "domain", skip_serializing_if = "Option::is_none")]
    pub referrer: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<&'a str>,
}

impl<'a> StatsLink<'a> {
    pub const fn new(range: &'a DateRange, path: Option<&'a str>) -> Self {
        Self {
            range,
            path,
            referrer: None,
            view: None,
        }
    }

    /// Attach the active dashboard tab so range/tab changes preserve each other.
    #[must_use]
    pub const fn with_view(mut self, view: PanelView) -> Self {
        self.view = view.url_value();
        self
    }

    /// Attach a referrer domain (used by the reverse, per-referrer view).
    #[must_use]
    pub const fn with_referrer(mut self, referrer: Option<&'a str>) -> Self {
        self.referrer = referrer;
        self
    }

    /// Builds a URL of the form `{base}?from=...&to=...&path=...`, omitting
    /// the query string entirely when no parameters are set.
    pub fn url(&self, base: &str) -> String {
        build_url(base, self)
    }

    /// Convenience: build the full set of filter buttons for the given
    /// endpoint, preserving this link's `path`/`referrer`/`view` while varying
    /// only the date range. `today` anchors the relative ranges; the link's own
    /// range is used to highlight the matching button.
    pub fn preset_buttons(&self, hx_endpoint: &str, today: Date) -> Vec<PresetButton> {
        let active = self.range.matched_preset();
        Preset::ALL
            .iter()
            .copied()
            .map(|preset| {
                let preset_range = preset.date_range(today);
                let link = StatsLink {
                    range: &preset_range,
                    path: self.path,
                    referrer: self.referrer,
                    view: self.view,
                };
                PresetButton {
                    key: preset.key(),
                    label: preset.label(),
                    hx_url: build_url(hx_endpoint, &link),
                    active: active == Some(preset),
                }
            })
            .collect()
    }
}

/// Serializes a [`StatsLink`] onto `base` as a query string, returning `base`
/// unchanged when no parameters are set.
fn build_url(base: &str, link: &StatsLink) -> String {
    let qs = serde_urlencoded::to_string(link).unwrap_or_default();
    if qs.is_empty() {
        base.to_string()
    } else {
        format!("{base}?{qs}")
    }
}

struct DataPoint<D> {
    x: D,
    y: u64,
}

impl<D> DataPoint<D>
where
    D: ContiguousDatePart,
{
    async fn all(
        state: &InnerAppState,
        filter: VisitFilter,
        now: OffsetDateTime,
        start_datetime: Option<PrimitiveDateTime>,
        end_datetime: Option<PrimitiveDateTime>,
        trunc_sql: &str,
    ) -> RespResult<Vec<Self>> {
        let start_utc = start_datetime.map(|pdt| local_to_utc(pdt, state.utc_offset));
        let end_utc = end_datetime.map(|pdt| local_to_utc(pdt, state.utc_offset));

        let sql = format!(
            r"SELECT {trunc_sql} AS trunc_registered_at,
            COUNT(registered_at) AS count FROM visits
            WHERE (? IS NULL OR path_id = ?)
              AND (? IS NULL OR referrer_id = ?)
              AND (? IS NULL OR registered_at >= ?)
              AND (? IS NULL OR registered_at < ?)
            GROUP BY trunc_registered_at
            ORDER BY trunc_registered_at"
        );

        let rows = sqlx::query_as::<Db, TruncDateCount>(&sql)
            .bind(filter.path_id)
            .bind(filter.path_id)
            .bind(filter.referrer_id)
            .bind(filter.referrer_id)
            .bind(start_utc)
            .bind(start_utc)
            .bind(end_utc)
            .bind(end_utc)
            .fetch_all(&state.pool)
            .await
            .ctx(StatusCode::INTERNAL_SERVER_ERROR)
            .log_msg("Failed to query chart data!")?;

        let now_date_part = D::from(now);
        let terminal_date_part = end_datetime.map(D::from).map_or(now_date_part, |ep| {
            if ep < now_date_part {
                ep
            } else {
                now_date_part
            }
        });

        let first_date_part = if let Some(start_datetime) = start_datetime {
            D::from(start_datetime)
        } else if let Some(row) = rows.first() {
            D::from(row.trunc_registered_at)
        } else {
            return Ok(vec![Self {
                x: terminal_date_part,
                y: 0,
            }]);
        };

        #[allow(clippy::option_if_let_else)]
        let additional_given_point = match rows.last() {
            Some(last_row) => {
                let last_row_date_part = D::from(last_row.trunc_registered_at);
                (terminal_date_part > last_row_date_part).then_some(Ok((terminal_date_part, 0)))
            }
            None => Some(Ok((terminal_date_part, 0))),
        };

        #[allow(clippy::cast_sign_loss)]
        let given_points = rows
            .into_iter()
            .map(|row| {
                let row_date_part = D::from(row.trunc_registered_at);
                Ok((row_date_part, row.count as u64))
            })
            .chain(additional_given_point);

        let mut aggregator = ChartDataAggregator::new(first_date_part);

        for given_point in given_points {
            let (row_date_part, count) = given_point?;

            if aggregator.next_date_part() < row_date_part {
                loop {
                    aggregator.push(0)?;
                    if aggregator.next_date_part() >= row_date_part {
                        break;
                    }
                }
            }
            aggregator.push(count)?;
        }

        Ok(aggregator.into_inner())
    }
}

/// Converts a local-time `PrimitiveDateTime` to UTC by subtracting the UTC offset.
/// This is the inverse of `InnerAppState::apply_utc_offset`.
pub fn local_to_utc(pdt: PrimitiveDateTime, offset: time::UtcOffset) -> PrimitiveDateTime {
    pdt - time::Duration::seconds(i64::from(offset.whole_seconds()))
}

fn to_chart_bars<D: ContiguousDatePart + std::fmt::Display>(
    points: Vec<DataPoint<D>>,
) -> Vec<ChartBar> {
    points
        .into_iter()
        .map(|p| ChartBar {
            label: p.x.to_string(),
            count: p.y,
        })
        .collect()
}

pub async fn build_chart(
    state: &'static InnerAppState,
    filter: VisitFilter,
    range: &DateRange,
    now: OffsetDateTime,
) -> RespResult<Vec<ChartBar>> {
    let start_dt = range.start_datetime();
    let end_dt = range.end_datetime();

    let whole_days = if let Some(days) = range.whole_days(now) {
        days
    } else {
        let Some(WholeDaysSinceFirstVisit {
            whole_days_since_first_visit,
            ..
        }) = WholeDaysSinceFirstVisit::build(state, filter, now).await?
        else {
            return Ok(vec![]);
        };
        whole_days_since_first_visit
    };

    if whole_days < 3 {
        let start = if start_dt.is_none() {
            Some(hour_data_start_datetime(now)?)
        } else {
            start_dt
        };
        let trunc = format!(
            "strftime('%Y-%m-%d %H:00:00', datetime(registered_at, '{}'))",
            state.posix_utc_offset_str
        );
        let points =
            DataPoint::<ContiguousHour>::all(state, filter, now, start, end_dt, &trunc).await?;
        Ok(to_chart_bars(points))
    } else if whole_days < 91 {
        let trunc = format!(
            "strftime('%Y-%m-%d 00:00:00', datetime(registered_at, '{}'))",
            state.posix_utc_offset_str
        );
        let points =
            DataPoint::<ContiguousDay>::all(state, filter, now, start_dt, end_dt, &trunc).await?;
        Ok(to_chart_bars(points))
    } else if whole_days < 3653 {
        let trunc = format!(
            "strftime('%Y-%m-01 00:00:00', datetime(registered_at, '{}'))",
            state.posix_utc_offset_str
        );
        let points =
            DataPoint::<ContiguousMonth>::all(state, filter, now, start_dt, end_dt, &trunc).await?;
        Ok(to_chart_bars(points))
    } else {
        let trunc = format!(
            "strftime('%Y-01-01 00:00:00', datetime(registered_at, '{}'))",
            state.posix_utc_offset_str
        );
        let points =
            DataPoint::<ContiguousYear>::all(state, filter, now, start_dt, end_dt, &trunc).await?;
        Ok(to_chart_bars(points))
    }
}

fn hour_data_start_datetime(now: OffsetDateTime) -> RespResult<PrimitiveDateTime> {
    let date = now.date() - Duration::days(2);
    let time = Time::from_hms(now.hour(), 0, 0)
        .ctx(StatusCode::INTERNAL_SERVER_ERROR)
        .log_msg("Failed to create Time for hour data!")?;
    Ok(PrimitiveDateTime::new(date, time))
}

#[cfg(test)]
mod tests {
    use super::DateRange;
    use time::macros::date;

    #[test]
    fn date_range_deserializes_from_query_string() {
        let r: DateRange = serde_urlencoded::from_str("from=2026-03-24&to=2026-04-23").unwrap();
        assert_eq!(r.from, Some(date!(2026 - 03 - 24)));
        assert_eq!(r.to, Some(date!(2026 - 04 - 23)));
    }

    #[test]
    fn date_range_deserializes_empty_and_missing_as_none() {
        let r: DateRange = serde_urlencoded::from_str("").unwrap();
        assert_eq!(r.from, None);
        assert_eq!(r.to, None);

        let r: DateRange = serde_urlencoded::from_str("from=&to=").unwrap();
        assert_eq!(r.from, None);
        assert_eq!(r.to, None);

        let r: DateRange = serde_urlencoded::from_str("from=2026-03-24").unwrap();
        assert_eq!(r.from, Some(date!(2026 - 03 - 24)));
        assert_eq!(r.to, None);
    }

    #[test]
    fn date_range_deserializes_invalid_as_none() {
        // Garbage input should not blow up the page; treat as no filter.
        let r: DateRange = serde_urlencoded::from_str("from=not-a-date&to=2026-04-23").unwrap();
        assert_eq!(r.from, None);
        assert_eq!(r.to, Some(date!(2026 - 04 - 23)));
    }

    #[test]
    fn date_range_query_string_round_trips() {
        let original = DateRange {
            from: Some(date!(2026 - 03 - 24)),
            to: Some(date!(2026 - 04 - 23)),
        };
        let qs = original.query_string();
        assert_eq!(qs, "from=2026-03-24&to=2026-04-23");

        let parsed: DateRange = serde_urlencoded::from_str(&qs).unwrap();
        assert_eq!(parsed.from, original.from);
        assert_eq!(parsed.to, original.to);
    }

    #[test]
    fn date_range_query_string_skips_none_fields() {
        let only_from = DateRange {
            from: Some(date!(2026 - 03 - 24)),
            to: None,
        };
        assert_eq!(only_from.query_string(), "from=2026-03-24");

        let only_to = DateRange {
            from: None,
            to: Some(date!(2026 - 04 - 23)),
        };
        assert_eq!(only_to.query_string(), "to=2026-04-23");

        let empty = DateRange::default();
        assert_eq!(empty.query_string(), "");
    }

    #[test]
    fn date_range_query_suffix_prefixes_ampersand() {
        let r = DateRange {
            from: Some(date!(2026 - 03 - 24)),
            to: Some(date!(2026 - 04 - 23)),
        };
        assert_eq!(r.query_suffix(), "&from=2026-03-24&to=2026-04-23");

        assert_eq!(DateRange::default().query_suffix(), "");
    }
}
