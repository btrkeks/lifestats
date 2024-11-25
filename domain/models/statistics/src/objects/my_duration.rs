use std::fmt;
use std::ops::Add;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct MyDuration {
    seconds: u64,
}

impl MyDuration {
    pub fn from_secs(seconds: u64) -> Self {
        Self {
            seconds
        }
    }

    pub fn from_mins(minutes: u64) -> Self {
        Self {
            seconds: minutes * 60
        }
    }
    pub fn from_hours(hours: u64) -> Self {
        Self {
            seconds: hours * 3600
        }
    }

    pub fn secs(&self) -> u64 {
        self.seconds
    }
}

impl std::str::FromStr for MyDuration {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(MyDuration::from_secs(0));
        }

        let len = s.len();
        let (value_str, unit) = s.split_at(len - 1);
        let value: u64 = value_str.parse().map_err(|_| anyhow!("Invalid number"))?;

        match unit {
            "s" => Ok(MyDuration::from_secs(value)),
            "m" => Ok(MyDuration::from_mins(value)),
            "h" => Ok(MyDuration::from_hours(value)),
            _ => Err(anyhow!("Invalid unit")),
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
        MyDuration {
            seconds: self.seconds + other.seconds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
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
}