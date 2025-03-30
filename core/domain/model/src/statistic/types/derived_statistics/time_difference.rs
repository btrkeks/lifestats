use crate::statistic::goals::amount_goal::AmountGoal;
use crate::statistic::repositories::goal_repository::GoalRepository;
use crate::statistic::traits::base_statistic::Statistic;
use crate::statistic::traits::integer_statistic::IntegerStatistic;
use crate::statistic::types::logable_statistics::point_in_time::PointInTimeStatistic;
use chrono::NaiveDate;
use std::ops::Sub;
use transfer_objects::statistic_metadata::StatisticMetadata;
use crate::objects::seconds::Seconds;
use crate::statistic::error::Result;

pub struct TimeDifferenceStatistic {
    metadata: StatisticMetadata,

    statistic0: PointInTimeStatistic,
    statistic1: PointInTimeStatistic,

    goals: Box<dyn GoalRepository<GoalType = <Self as Statistic>::GoalType>>,
    /// The current default goal
    default_goal: Option<<Self as Statistic>::GoalType>,
    /// The date from which the current default goal became active (including)
    default_goal_start: NaiveDate,
}

impl Statistic for TimeDifferenceStatistic {
    type LogValueType = Seconds;
    type GoalType = AmountGoal<Self::LogValueType>;

    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }

    fn get_log_value(&self, date: NaiveDate) -> Result<Option<Self::LogValueType>> {
        // TODO: It should be possible to write this cleaner
        let x0 = if let Some(x0) = self.statistic0.get_log_value(date)? {
            x0
        } else {
            return Ok(None);
        };
        let x1 = if let Some(x1) = self.statistic1.get_log_value(date)? {
            x1
        } else {
            return Ok(None);
        };

        Ok(Some(if x1 >= x0 { Seconds::from(x1.sub(x0)) } else { Seconds::from(x0.sub(x0)) }))
    }

    fn get_goal(&self, date: NaiveDate) -> Result<Option<Self::GoalType>> {
        self.goals.get(date)
    }

    fn set_goal(&self, date: NaiveDate, goal: Self::GoalType) -> Result<()> {
        self.goals.set(date, goal)
    }

    fn get_default_goal(&self) -> Option<&Self::GoalType> {
        self.default_goal.as_ref()
    }

    fn _set_default_goal(&mut self, goal: Self::GoalType, start_date: NaiveDate) {
        self.default_goal.replace(goal);
        self.default_goal_start = start_date;
    }

    fn get_default_goal_valid_since(&self) -> NaiveDate {
        self.default_goal_start
    }
}

impl IntegerStatistic for TimeDifferenceStatistic {
    fn value_into_i64(&self, value: Self::LogValueType) -> i64 {
        value.into()
    }
}
