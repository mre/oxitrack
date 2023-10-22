use axum_ctx::{RespErr, Status};
use serde::{Serialize, Serializer};
use std::fmt;
use time::{Date, Month, OffsetDateTime};

pub trait ContiguousDatePart: From<OffsetDateTime> + Serialize + Copy + PartialEq {
    fn next(&mut self) -> Result<(), RespErr>;

    fn date_truncation() -> &'static str;
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct ContiguousYear(i32);

impl From<OffsetDateTime> for ContiguousYear {
    fn from(datetime: OffsetDateTime) -> Self {
        Self(datetime.year())
    }
}

impl Serialize for ContiguousYear {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&self.0)
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

#[derive(PartialEq, Eq, Clone, Copy)]
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

impl Serialize for ContiguousMonth {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&format_args!("{}.{:02}", self.year, self.month as u8))
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

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct ContiguousDay(Date);

impl From<OffsetDateTime> for ContiguousDay {
    fn from(datetime: OffsetDateTime) -> Self {
        Self(datetime.date())
    }
}

impl Serialize for ContiguousDay {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&self.0)
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

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct ContiguousHour {
    day: ContiguousDay,
    hour: u8,
}

impl From<OffsetDateTime> for ContiguousHour {
    fn from(date: OffsetDateTime) -> Self {
        Self {
            day: ContiguousDay::from(date),
            hour: date.hour(),
        }
    }
}

impl Serialize for ContiguousHour {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(&format_args!("{} {:02}:00", self.day, self.hour))
    }
}

impl ContiguousDatePart for ContiguousHour {
    fn next(&mut self) -> Result<(), RespErr> {
        match self.hour {
            0..=22 => {
                self.hour += 1;
                Ok(())
            }
            _ => {
                self.hour = 0;
                self.day.next()
            }
        }
    }

    fn date_truncation() -> &'static str {
        "hour"
    }
}
