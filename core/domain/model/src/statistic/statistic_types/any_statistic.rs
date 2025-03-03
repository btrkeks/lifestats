use crate::statistic::integer_statistic::IntegerStatistic;
use crate::statistic::statistic::Statistic;
use crate::statistic::statistic_types::habit::HabitStatistic;
use crate::statistic::statistic_types::integer_amount::IntegerAmountStatistic;
use crate::statistic::statistic_types::point_in_time::PointInTimeStatistic;
use crate::statistic::statistic_types::time_amount::TimeAmountStatistic;
use crate::statistic::statistic_types::time_spans::TimeSpansStatistic;
use anyhow::{anyhow, Result};
use chrono::NaiveDate;
use transfer_objects::raw_goal::RawGoal;
use transfer_objects::statistic_id::StatisticId;
use transfer_objects::statistic_metadata::StatisticMetadata;

pub enum StatisticType {
    IntegerAmount,
    TimeAmount,
    PointInTime,
    TimeSpans,
    Habit,
}

#[derive(Clone)]
pub enum AnyStatistic {
    IntegerAmount(IntegerAmountStatistic),
    TimeAmount(TimeAmountStatistic),
    PointInTime(PointInTimeStatistic),
    TimeSpans(TimeSpansStatistic),
    Habit(HabitStatistic),
}

impl AnyStatistic {
    pub fn into_time_spans(self) -> Option<TimeSpansStatistic> {
        match self {
            AnyStatistic::TimeSpans(s) => Some(s),
            _ => None,
        }
    }

    pub fn into_integer_statistic(self) -> Result<Box<dyn IntegerStatistic>> {
        match self {
            AnyStatistic::IntegerAmount(s) => Ok(Box::new(s)),
            AnyStatistic::TimeSpans(s) => Ok(Box::new(s)),
            AnyStatistic::TimeAmount(s) => Ok(Box::new(s)),
            AnyStatistic::PointInTime(s) => Ok(Box::new(s)),
            _ => Err(anyhow!("Not an integer statistic")),
        }
    }

    pub fn id(&self) -> StatisticId {
        match self {
            AnyStatistic::IntegerAmount(s) => s.id(),
            AnyStatistic::TimeAmount(s) => s.id(),
            AnyStatistic::PointInTime(s) => s.id(),
            AnyStatistic::TimeSpans(s) => s.id(),
            AnyStatistic::Habit(s) => s.id(),
        }
    }

    pub fn get_metadata(&self) -> StatisticMetadata {
        match self {
            AnyStatistic::IntegerAmount(s) => s.get_metadata().clone(),
            AnyStatistic::TimeAmount(s) => s.get_metadata().clone(),
            AnyStatistic::PointInTime(s) => s.get_metadata().clone(),
            AnyStatistic::TimeSpans(s) => s.get_metadata().clone(),
            AnyStatistic::Habit(s) => s.get_metadata().clone(),
        }
    }

    pub fn get_name(&self) -> &str {
        match self {
            AnyStatistic::IntegerAmount(s) => s.get_name(),
            AnyStatistic::TimeAmount(s) => s.get_name(),
            AnyStatistic::PointInTime(s) => s.get_name(),
            AnyStatistic::TimeSpans(s) => s.get_name(),
            AnyStatistic::Habit(s) => s.get_name(),
        }
    }

    pub fn get_category(&self) -> &str {
        match self {
            AnyStatistic::IntegerAmount(s) => s.get_category(),
            AnyStatistic::TimeAmount(s) => s.get_category(),
            AnyStatistic::PointInTime(s) => s.get_category(),
            AnyStatistic::TimeSpans(s) => s.get_category(),
            AnyStatistic::Habit(s) => s.get_category(),
        }
    }

    pub fn log_from_string(
        &self,
        date: NaiveDate,
        value: &str,
        update_policy: Option<&str>,
    ) -> Result<()> {
        match self {
            AnyStatistic::IntegerAmount(s) => s.log_from_string(date, value, update_policy),
            AnyStatistic::TimeAmount(s) => s.log_from_string(date, value, update_policy),
            AnyStatistic::PointInTime(s) => s.log_from_string(date, value, update_policy),
            AnyStatistic::TimeSpans(s) => s.log_from_string(date, value, update_policy),
            AnyStatistic::Habit(s) => s.log_from_string(date, value, update_policy),
        }
    }

    pub fn set_default_goal_from_string(
        &mut self,
        target_string: Option<&str>,
        extra_data_1: Option<&str>,
    ) -> Result<()> {
        match self {
            AnyStatistic::IntegerAmount(s) => {
                s.set_new_default_goal_from_string(target_string, extra_data_1)
            }
            AnyStatistic::TimeAmount(s) => {
                s.set_new_default_goal_from_string(target_string, extra_data_1)
            }
            AnyStatistic::PointInTime(s) => {
                s.set_new_default_goal_from_string(target_string, extra_data_1)
            }
            AnyStatistic::TimeSpans(s) => {
                s.set_new_default_goal_from_string(target_string, extra_data_1)
            }
            AnyStatistic::Habit(s) => {
                s.set_new_default_goal_from_string(target_string, extra_data_1)
            }
        }
    }

    pub fn set_goal_for_date_from_string(
        &mut self,
        date: NaiveDate,
        target_string: Option<&str>,
        extra_data_1: Option<&str>,
    ) -> Result<()> {
        match self {
            AnyStatistic::IntegerAmount(s) => {
                s.set_goal_for_date_from_string(date, target_string, extra_data_1)
            }
            AnyStatistic::TimeAmount(s) => {
                s.set_goal_for_date_from_string(date, target_string, extra_data_1)
            }
            AnyStatistic::PointInTime(s) => {
                s.set_goal_for_date_from_string(date, target_string, extra_data_1)
            }
            AnyStatistic::TimeSpans(s) => {
                s.set_goal_for_date_from_string(date, target_string, extra_data_1)
            }
            AnyStatistic::Habit(s) => {
                s.set_goal_for_date_from_string(date, target_string, extra_data_1)
            }
        }
    }

    pub fn get_type_name(&self) -> &'static str {
        match self {
            AnyStatistic::IntegerAmount(_) => "IntegerAmount",
            AnyStatistic::TimeAmount(_) => "TimeAmount",
            AnyStatistic::PointInTime(_) => "PointInTime",
            AnyStatistic::TimeSpans(_) => "TimeSpans",
            AnyStatistic::Habit(_) => "Habit",
        }
    }

    pub fn get_type(&self) -> StatisticType {
        match self {
            AnyStatistic::IntegerAmount(_) => StatisticType::IntegerAmount,
            AnyStatistic::TimeAmount(_) => StatisticType::TimeAmount,
            AnyStatistic::PointInTime(_) => StatisticType::PointInTime,
            AnyStatistic::TimeSpans(_) => StatisticType::TimeSpans,
            AnyStatistic::Habit(_) => StatisticType::Habit,
        }
    }

    pub fn get_raw_default_goal(&mut self) -> Result<Option<RawGoal>> {
        match self {
            AnyStatistic::IntegerAmount(s) => s.get_raw_default_goal(),
            AnyStatistic::TimeAmount(s) => s.get_raw_default_goal(),
            AnyStatistic::PointInTime(s) => s.get_raw_default_goal(),
            AnyStatistic::TimeSpans(s) => s.get_raw_default_goal(),
            AnyStatistic::Habit(s) => s.get_raw_default_goal(),
        }
    }

    pub fn get_raw_goal_for_date(&mut self, date: NaiveDate) -> Result<Option<RawGoal>> {
        match self {
            AnyStatistic::IntegerAmount(s) => s.get_raw_goal_for_date(date),
            AnyStatistic::TimeAmount(s) => s.get_raw_goal_for_date(date),
            AnyStatistic::PointInTime(s) => s.get_raw_goal_for_date(date),
            AnyStatistic::TimeSpans(s) => s.get_raw_goal_for_date(date),
            AnyStatistic::Habit(s) => s.get_raw_goal_for_date(date),
        }
    }
}
