use std::fmt;
use std::ops::Add;
use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use crate::statistics::goal::Goal;
use crate::statistics::type_definitions::{Statistic, StatisticMetadata, StatisticTypeSpecific};
use crate::statistics::update_policy::AmountUpdatePolicy;

// TODO: Could actually just wrap an Integer amount statistic with seconds as value

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeAmountStatistic {
    metadata: StatisticMetadata,
    goal: Option<TimeAmountGoal>,
}

impl Statistic for TimeAmountStatistic {
    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }
}

impl StatisticTypeSpecific for TimeAmountStatistic {
    type LogValue = MyDuration;
    type RetrieveValue = MyDuration;
    type UpdatePolicy = AmountUpdatePolicy;
    type GoalType = TimeAmountGoal;

    fn get_goal(&self) -> &Option<Self::GoalType> {
        &self.goal
    }

    // fn parse_string_to_value(&self, string: &str) -> anyhow::Result<Self::LogValue> {
    //     parse_duration(string)
    //         .with_context(|| anyhow!("Parsing {} into a duration for id {:?}", string, self.get_id()))
    // }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeAmountGoal {
    target: MyDuration,
    is_good: bool,
}

impl Goal<MyDuration> for TimeAmountGoal {
    fn get_percentage_achieved(&self, value: &MyDuration) -> f64 {
        if self.is_good {
            value.secs() as f64 / self.target.secs() as f64
        } else {
            self.target.secs() as f64 / value.secs() as f64
        }
    }
}

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
