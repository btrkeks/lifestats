use anyhow::anyhow;
use chrono::{NaiveTime, TimeDelta, Timelike};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// A wrapper for NaiveTime that implements a custom de-/serialization
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Copy)]
pub struct MyTime(pub NaiveTime);

impl MyTime {
    pub fn new(time: NaiveTime) -> Self {
        Self(time)
    }

    pub fn into_inner(self) -> NaiveTime {
        self.0
    }

    pub fn num_seconds_from_midnight(&self) -> u32 {
        self.0.num_seconds_from_midnight()
    }

    pub fn second(&self) -> u32 {
        self.0.second()
    }

    pub fn signed_duration_since(&self, other: MyTime) -> TimeDelta {
        self.0.signed_duration_since(other.0)
    }
}

impl Serialize for MyTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.format("%H:%M:%S").to_string())
    }
}

impl<'de> Deserialize<'de> for MyTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let time_str = String::deserialize(deserializer)?;
        NaiveTime::parse_from_str(&time_str, "%H:%M:%S")
            .map(MyTime)
            .map_err(serde::de::Error::custom)
    }
}

// Implement Display and FromStr to satisfy StatisticTemplate constraints
impl fmt::Display for MyTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%H:%M:%S"))
    }
}

impl FromStr for MyTime {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let supported_formats = [
            "%H:%M:%S",    // 14:30:59
            "%H:%M",       // 14:30
            "%H:%M:%S%.f", // 14:30:59.123 (used by NaiveTime)
            "%I:%M %p",    // 2:30 PM
            "%I:%M:%S %p", // 2:30:59 PM
            "%r",          // 02:30:59 PM
        ];

        for format in &supported_formats {
            if let Ok(time) = NaiveTime::parse_from_str(s, format) {
                return Ok(MyTime(time));
            }
        }

        Err(anyhow!("Unsupported time format"))
    }
}
