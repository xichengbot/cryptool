use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, LocalResult, NaiveDateTime, Offset, TimeZone, Utc};
use chrono_tz::Asia::Shanghai;
use serde::{Deserialize, Serialize};

use crate::{ErrorCode, QueryError};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TimeUnit {
    #[serde(rename = "s")]
    Seconds,
    #[serde(rename = "ms")]
    Millis,
    #[serde(rename = "us")]
    Micros,
}

impl fmt::Display for TimeUnit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Seconds => "s",
            Self::Millis => "ms",
            Self::Micros => "us",
        })
    }
}

impl FromStr for TimeUnit {
    type Err = QueryError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "s" => Ok(Self::Seconds),
            "ms" => Ok(Self::Millis),
            "us" => Ok(Self::Micros),
            _ => Err(invalid(format!(
                "invalid time unit '{input}', expected s, ms, or us"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Timestamp {
    #[serde(with = "i64_string")]
    pub value: i64,
    pub unit: TimeUnit,
}

impl Timestamp {
    pub fn to_micros(self) -> Result<i64, QueryError> {
        let multiplier = match self.unit {
            TimeUnit::Seconds => 1_000_000,
            TimeUnit::Millis => 1_000,
            TimeUnit::Micros => 1,
        };
        self.value.checked_mul(multiplier).ok_or_else(|| {
            invalid(format!(
                "timestamp {} {} cannot be represented in microseconds",
                self.value, self.unit
            ))
        })
    }
}

pub fn parse_time(input: &str, unit: Option<TimeUnit>) -> Result<Timestamp, QueryError> {
    if input.is_empty() {
        return Err(invalid("time value must not be empty"));
    }

    if let Some(unit) = unit {
        let value = input
            .parse::<i64>()
            .map_err(|_| invalid(format!("invalid numeric timestamp '{input}'")))?;
        let timestamp = Timestamp { value, unit };
        timestamp.to_micros()?;
        return Ok(timestamp);
    }

    if let Some(digit_count) = signed_digit_count(input) {
        let unit = match digit_count {
            10 => TimeUnit::Seconds,
            13 => TimeUnit::Millis,
            16 => TimeUnit::Micros,
            _ => {
                return Err(invalid(format!(
                    "numeric timestamp '{input}' is ambiguous; use 10, 13, or 16 digits or specify a unit"
                )));
            }
        };
        let value = input
            .parse::<i64>()
            .map_err(|_| invalid(format!("invalid numeric timestamp '{input}'")))?;
        let timestamp = Timestamp { value, unit };
        timestamp.to_micros()?;
        return Ok(timestamp);
    }

    reject_sub_microsecond_precision(input)?;

    if let Ok(value) = DateTime::parse_from_rfc3339(input) {
        return Ok(Timestamp {
            value: value.timestamp_micros(),
            unit: TimeUnit::Micros,
        });
    }

    let format = if input.contains('.') {
        "%Y-%m-%d %H:%M:%S%.f"
    } else {
        "%Y-%m-%d %H:%M:%S"
    };
    let naive = NaiveDateTime::parse_from_str(input, format)
        .map_err(|_| invalid(format!("invalid time value '{input}'")))?;
    let value = match Shanghai.from_local_datetime(&naive) {
        LocalResult::Single(value) => value,
        LocalResult::Ambiguous(_, _) => {
            return Err(invalid(format!("ambiguous Asia/Shanghai time '{input}'")));
        }
        LocalResult::None => {
            return Err(invalid(format!("nonexistent Asia/Shanghai time '{input}'")));
        }
    };
    Ok(Timestamp {
        value: value.timestamp_micros(),
        unit: TimeUnit::Micros,
    })
}

pub fn format_utc(value: Timestamp) -> Result<String, QueryError> {
    let datetime = to_datetime(value)?;
    Ok(match value.unit {
        TimeUnit::Seconds => datetime.format("%Y-%m-%d %H:%M:%SZ").to_string(),
        TimeUnit::Millis => datetime.format("%Y-%m-%d %H:%M:%S%.3fZ").to_string(),
        TimeUnit::Micros => datetime.format("%Y-%m-%d %H:%M:%S%.6fZ").to_string(),
    })
}

pub fn format_beijing(value: Timestamp) -> Result<String, QueryError> {
    let datetime = to_datetime(value)?.with_timezone(&Shanghai);
    let timestamp_format = match value.unit {
        TimeUnit::Seconds => "%Y-%m-%d %H:%M:%S",
        TimeUnit::Millis => "%Y-%m-%d %H:%M:%S%.3f",
        TimeUnit::Micros => "%Y-%m-%d %H:%M:%S%.6f",
    };
    let offset_format = if datetime.offset().fix().local_minus_utc() % 60 == 0 {
        "%:z"
    } else {
        "%::z"
    };
    Ok(format!(
        "{}{}",
        datetime.format(timestamp_format),
        datetime.format(offset_format)
    ))
}

fn to_datetime(value: Timestamp) -> Result<DateTime<Utc>, QueryError> {
    let (seconds, nanos) = match value.unit {
        TimeUnit::Seconds => (value.value, 0),
        TimeUnit::Millis => (
            value.value.div_euclid(1_000),
            value.value.rem_euclid(1_000) as u32 * 1_000_000,
        ),
        TimeUnit::Micros => (
            value.value.div_euclid(1_000_000),
            value.value.rem_euclid(1_000_000) as u32 * 1_000,
        ),
    };
    DateTime::from_timestamp(seconds, nanos).ok_or_else(|| {
        invalid(format!(
            "timestamp {} {} is outside the supported date range",
            value.value, value.unit
        ))
    })
}

fn signed_digit_count(input: &str) -> Option<usize> {
    let digits = input.strip_prefix('-').unwrap_or(input);
    (!digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())).then_some(digits.len())
}

fn reject_sub_microsecond_precision(input: &str) -> Result<(), QueryError> {
    let Some((_, fractional)) = input.split_once('.') else {
        return Ok(());
    };
    let digits = fractional
        .bytes()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    if digits > 6 {
        return Err(invalid("fractional seconds must not exceed 6 digits"));
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> QueryError {
    QueryError::new(ErrorCode::InvalidInput, message)
}

mod i64_string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &i64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<i64, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}
