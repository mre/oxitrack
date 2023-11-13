use time::{Duration, OffsetDateTime};

#[derive(Clone)]
pub struct StartDatetime {
    pub start: OffsetDateTime,
    pub now: OffsetDateTime,
}

impl StartDatetime {
    pub fn from_sub_duration(duration: Duration) -> Self {
        let now = OffsetDateTime::now_utc();

        Self {
            start: now - duration,
            now,
        }
    }
}

pub struct OptionStartDateTime {
    pub start: Option<OffsetDateTime>,
    pub now: OffsetDateTime,
}

impl From<Option<StartDatetime>> for OptionStartDateTime {
    fn from(opt: Option<StartDatetime>) -> Self {
        match opt {
            Some(StartDatetime { start, now }) => Self {
                start: Some(start),
                now,
            },
            None => Self {
                start: None,
                now: OffsetDateTime::now_utc(),
            },
        }
    }
}
