use std::fmt;
use std::str::FromStr;

use crate::cqrs::Command;
use crate::error::ApplicationError;
use lifestats_core::shared::ids::StatisticId;

/// Strongly typed representation of the statistic kind to create.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatisticTypeDto {
    IntegerAmount,
    Habit,
    PointInTime,
    TimeSpans,
    Summing,
    TimeDifference,
}

impl StatisticTypeDto {
    pub fn as_str(&self) -> &'static str {
        match self {
            StatisticTypeDto::IntegerAmount => "IntegerAmount",
            StatisticTypeDto::Habit => "Habit",
            StatisticTypeDto::PointInTime => "PointInTime",
            StatisticTypeDto::TimeSpans => "TimeSpans",
            StatisticTypeDto::Summing => "Summing",
            StatisticTypeDto::TimeDifference => "TimeDifference",
        }
    }
}

impl fmt::Display for StatisticTypeDto {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct StatisticTypeParseError {
    invalid_value: String,
}

impl StatisticTypeParseError {
    pub fn new(invalid_value: impl Into<String>) -> Self {
        Self {
            invalid_value: invalid_value.into(),
        }
    }

    pub fn to_application_error(self) -> ApplicationError {
        ApplicationError::InvalidInput(format!("Unknown statistic_type: {}", self.invalid_value))
    }
}

impl fmt::Display for StatisticTypeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown statistic_type: {}", self.invalid_value)
    }
}

impl std::error::Error for StatisticTypeParseError {}

impl FromStr for StatisticTypeDto {
    type Err = StatisticTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "IntegerAmount" => Ok(StatisticTypeDto::IntegerAmount),
            "Habit" => Ok(StatisticTypeDto::Habit),
            "PointInTime" => Ok(StatisticTypeDto::PointInTime),
            "TimeSpans" => Ok(StatisticTypeDto::TimeSpans),
            "Summing" => Ok(StatisticTypeDto::Summing),
            "TimeDifference" => Ok(StatisticTypeDto::TimeDifference),
            _ => Err(StatisticTypeParseError::new(s)),
        }
    }
}

/// Command to create a new statistic metadata entry.
pub struct CreateStatisticCommand {
    pub name: String,
    pub category: String,
    pub statistic_type: StatisticTypeDto,
    pub source_statistic_ids: Option<Vec<StatisticId>>,
}

impl Command for CreateStatisticCommand {
    type Result = lifestats_core::statistic::any_statistic::AnyStatistic;
}
