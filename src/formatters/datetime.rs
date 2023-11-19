use serde::{Serialize, Serializer};
use time::PrimitiveDateTime;

pub struct DateTimeFormatter(pub PrimitiveDateTime);

impl Serialize for DateTimeFormatter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&format_args!("{}T{}", self.0.date(), self.0.time()))
    }
}
