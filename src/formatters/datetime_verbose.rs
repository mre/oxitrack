use std::fmt;
use time::OffsetDateTime;

pub struct DateTimeVerboseFormatter(pub OffsetDateTime);

impl fmt::Display for DateTimeVerboseFormatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {:02}:{:02} UTC{}",
            self.0.date(),
            self.0.hour(),
            self.0.minute(),
            self.0.offset(),
        )
    }
}
