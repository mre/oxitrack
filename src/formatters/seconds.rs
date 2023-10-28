use std::fmt;

pub struct SecondsFormatter(pub u64);

impl fmt::Display for SecondsFormatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let minutes = self.0 / 60;

        if minutes > 0 {
            write!(f, "{}min {:02}s", self.0 / 60, self.0 % 60)
        } else {
            write!(f, "{:02}s", self.0 % 60)
        }
    }
}
