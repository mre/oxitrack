use serde::{Serialize, Serializer};
use time::OffsetDateTime;

pub struct DatetimeFormatter(pub OffsetDateTime);

impl Serialize for DatetimeFormatter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&format_args!("{}T{}", self.0.date(), self.0.time()))
    }
}
