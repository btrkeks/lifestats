use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Add;

#[derive(Debug)]
enum Unit {
    Seconds,
    Minutes,
    Hours,
}

impl Unit {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "s" => Some(Unit::Seconds),
            "m" => Some(Unit::Minutes),
            "h" => Some(Unit::Hours),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MyDuration(u64); // Wrapper around seconds

impl MyDuration {
    pub fn from_secs(seconds: u64) -> Self {
        Self(seconds)
    }

    pub fn from_mins(minutes: u64) -> Self {
        Self(minutes * 60)
    }
    pub fn from_hours(hours: u64) -> Self {
        Self(hours * 3600)
    }

    pub fn secs(&self) -> u64 {
        self.0
    }
}

impl std::str::FromStr for MyDuration {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self, Self::Err> {
        let (value_str, unit_str) = s
            .strip_suffix(|c: char| c.is_alphabetic())
            .map(|v| (v, &s[v.len()..]))
            .ok_or_else(|| anyhow!("String must end with a unit (s, m, or h)"))?;

        let value: u64 = value_str
            .parse()
            .with_context(|| anyhow!("Expected a number, got: '{}'", value_str))?;

        let unit = Unit::from_str(unit_str).ok_or_else(|| {
            anyhow!("Invalid unit! Expected s (seconds), m (minutes) or h (hours)")
        })?;

        match unit {
            Unit::Seconds => Ok(MyDuration::from_secs(value)),
            Unit::Minutes => Ok(MyDuration::from_mins(value)),
            Unit::Hours => Ok(MyDuration::from_hours(value)),
        }
    }
}

impl fmt::Display for MyDuration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}s", self.secs())
    }
}

impl Add for MyDuration {
    type Output = MyDuration;

    fn add(self, other: MyDuration) -> MyDuration {
        MyDuration::from_secs(self.secs() + other.secs())
    }
}

impl Serialize for MyDuration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize just the seconds as a number
        serializer.serialize_u64(self.secs())
    }
}

impl<'de> Deserialize<'de> for MyDuration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize directly into seconds
        let seconds = u64::deserialize(deserializer)?;
        Ok(MyDuration::from_secs(seconds))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration_lowercase() {
        let parsed1: MyDuration = "30s".parse().unwrap();
        assert_eq!(parsed1, MyDuration::from_secs(30));
        let parsed2: MyDuration = "45m".parse().unwrap();
        assert_eq!(parsed2, MyDuration::from_mins(45));
        let parsed3: MyDuration = "2h".parse().unwrap();
        assert_eq!(parsed3, MyDuration::from_hours(2));

        let parsed4: Result<MyDuration, _> = "30".parse();
        assert!(parsed4.is_err());
        let parsed5: Result<MyDuration, _> = "30x".parse();
        assert!(parsed5.is_err());
        let parsed6: Result<MyDuration, _> = "invalid".parse();
        assert!(parsed6.is_err());
    }

    #[test]
    fn test_not_specifying_a_unit_should_produce_an_error() {
        let parsed: Result<MyDuration, _> = "30".parse();
        assert!(parsed.is_err());

        let parsed: Result<MyDuration, _> = "239065025".parse();
        assert!(parsed.is_err());
    }

    #[test]
    fn test_wrong_unit_should_produce_an_error() {
        let parsed: Result<MyDuration, _> = "30x".parse();
        assert!(parsed.is_err());

        let parsed: Result<MyDuration, _> = "181U".parse();
        assert!(parsed.is_err());
    }

    #[test]
    fn test_letters_should_produce_an_error() {
        let parsed: Result<MyDuration, _> = "123a01s".parse();
        assert!(parsed.is_err());

        let parsed: Result<MyDuration, _> = "18b9M".parse();
        assert!(parsed.is_err());

        let parsed: Result<MyDuration, _> = "181SS".parse();
        assert!(parsed.is_err());

        let parsed: Result<MyDuration, _> = "m1235S".parse();
        assert!(parsed.is_err());
    }

    #[test]
    fn test_parse_duration_uppercase() {
        let parsed1: MyDuration = "1263S".parse().unwrap();
        assert_eq!(parsed1, MyDuration::from_secs(1263));
        let parsed2: MyDuration = "86765M".parse().unwrap();
        assert_eq!(parsed2, MyDuration::from_mins(86765));
        let parsed3: MyDuration = "23408H".parse().unwrap();
        assert_eq!(parsed3, MyDuration::from_hours(23408));
    }
}
