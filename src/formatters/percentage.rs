use std::fmt;

pub struct PercentageFormatter(pub f64);

impl fmt::Display for PercentageFormatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}", self.0)
    }
}
