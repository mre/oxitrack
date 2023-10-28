use serde::{Serialize, Serializer};
use time::UtcOffset;

pub struct UtcOffsetFormatter(pub UtcOffset);

impl Serialize for UtcOffsetFormatter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&self.0)
    }
}
