use axum_ctx::{RespErr, Status};
use serde::{Serialize, Serializer};
use std::{cmp::Ordering, fmt};
use time::{Date, Month, OffsetDateTime, PrimitiveDateTime};

pub trait ContiguousDatePart:
    From<OffsetDateTime> + From<PrimitiveDateTime> + Serialize + Copy + Eq + Ord
{
    fn next(&mut self) -> Result<(), RespErr>;

    fn date_truncation() -> &'static str;
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContiguousYear(i32);

impl From<OffsetDateTime> for ContiguousYear {
    fn from(datetime: OffsetDateTime) -> Self {
        Self(datetime.year())
    }
}

impl From<PrimitiveDateTime> for ContiguousYear {
    fn from(datetime: PrimitiveDateTime) -> Self {
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ContiguousMonth {
    year: i32,
    month: Month,
}

impl Ord for ContiguousMonth {
    fn cmp(&self, other: &Self) -> Ordering {
        let self_month = self.month as u8;
        let other_month = other.month as u8;

        match self.year.cmp(&other.year) {
            Ordering::Equal => self_month.cmp(&other_month),
            o => o,
        }
    }
}

impl PartialOrd for ContiguousMonth {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl From<OffsetDateTime> for ContiguousMonth {
    fn from(datetime: OffsetDateTime) -> Self {
        Self {
            year: datetime.year(),
            month: datetime.month(),
        }
    }
}

impl From<PrimitiveDateTime> for ContiguousMonth {
    fn from(datetime: PrimitiveDateTime) -> Self {
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContiguousDay(Date);

impl From<OffsetDateTime> for ContiguousDay {
    fn from(datetime: OffsetDateTime) -> Self {
        Self(datetime.date())
    }
}

impl From<PrimitiveDateTime> for ContiguousDay {
    fn from(datetime: PrimitiveDateTime) -> Self {
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContiguousHour {
    day: ContiguousDay,
    hour: u8,
}

impl From<OffsetDateTime> for ContiguousHour {
    fn from(datetime: OffsetDateTime) -> Self {
        Self {
            day: ContiguousDay::from(datetime),
            hour: datetime.hour(),
        }
    }
}

impl From<PrimitiveDateTime> for ContiguousHour {
    fn from(datetime: PrimitiveDateTime) -> Self {
        Self {
            day: ContiguousDay::from(datetime),
            hour: datetime.hour(),
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
        if let 0..=22 = self.hour {
            self.hour += 1;
            Ok(())
        } else {
            self.hour = 0;
            self.day.next()
        }
    }

    fn date_truncation() -> &'static str {
        "hour"
    }
}
