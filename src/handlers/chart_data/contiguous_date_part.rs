use axum_ctx::{RespErr, Status};
use std::fmt;
use time::{Date, Month, OffsetDateTime};

pub trait ContiguousDatePart: From<OffsetDateTime> + PartialEq + fmt::Display {
    fn next(&mut self) -> Result<(), RespErr>;

    fn date_truncation() -> &'static str;
}

#[derive(PartialEq, Eq)]
pub struct ContiguousYear(i32);

impl From<OffsetDateTime> for ContiguousYear {
    fn from(datetime: OffsetDateTime) -> Self {
        Self(datetime.year())
    }
}

impl fmt::Display for ContiguousYear {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl ContiguousDatePart for ContiguousYear {
    fn next(&mut self) -> Result<(), RespErr> {
        self.0 += 1;

        Ok(())
    }

    fn date_truncation() -> &'static str {
        "year"
    }
}

#[derive(PartialEq, Eq)]
pub struct ContiguousMonth {
    year: i32,
    month: Month,
}

impl From<OffsetDateTime> for ContiguousMonth {
    fn from(datetime: OffsetDateTime) -> Self {
        Self {
            year: datetime.year(),
            month: datetime.month(),
        }
    }
}

impl fmt::Display for ContiguousMonth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.month.fmt(f)
    }
}

impl ContiguousDatePart for ContiguousMonth {
    fn next(&mut self) -> Result<(), RespErr> {
        match self.month {
            Month::January => self.month = Month::February,
            Month::February => self.month = Month::March,
            Month::March => self.month = Month::April,
            Month::April => self.month = Month::May,
            Month::May => self.month = Month::June,
            Month::June => self.month = Month::July,
            Month::July => self.month = Month::August,
            Month::August => self.month = Month::September,
            Month::September => self.month = Month::October,
            Month::October => self.month = Month::November,
            Month::November => self.month = Month::December,
            Month::December => {
                self.year += 1;
                self.month = Month::January;
            }
        }

        Ok(())
    }

    fn date_truncation() -> &'static str {
        "month"
    }
}

#[derive(PartialEq, Eq)]
pub struct ContiguousDay(Date);

impl From<OffsetDateTime> for ContiguousDay {
    fn from(datetime: OffsetDateTime) -> Self {
        Self(datetime.date())
    }
}

impl fmt::Display for ContiguousDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl ContiguousDatePart for ContiguousDay {
    fn next(&mut self) -> Result<(), RespErr> {
        match self.0.next_day() {
            Some(day) => {
                self.0 = day;
                Ok(())
            }
            None => Err(RespErr::new(Status::Internal).log_msg("Failed to get the next day!")),
        }
    }

    fn date_truncation() -> &'static str {
        "day"
    }
}
