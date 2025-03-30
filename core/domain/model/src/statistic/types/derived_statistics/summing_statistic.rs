use crate::statistic::error::{Result, StatisticError};
use crate::statistic::goals::amount_goal::AmountGoal;
use crate::statistic::repositories::goal_repository::GoalRepository;
use crate::statistic::traits::base_statistic::Statistic;
use crate::statistic::traits::integer_statistic::IntegerStatistic;
use crate::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;
use chrono::{NaiveDate, TimeDelta};
use transfer_objects::statistic_metadata::StatisticMetadata;
use crate::objects::seconds::Seconds;

/// This statistic takes a list of time spans statistics and sum the total time
pub struct SummingStatistic {
    metadata: StatisticMetadata,

    statistics: Vec<TimeSpansStatistic>,

    goals: Box<dyn GoalRepository<GoalType = <Self as Statistic>::GoalType>>,
    /// The current default goal
    default_goal: Option<<Self as Statistic>::GoalType>,
    /// The date from which the current default goal became active (including)
    default_goal_start: NaiveDate,
}

impl Statistic for SummingStatistic {
    type LogValueType = Seconds;
    type GoalType = AmountGoal<Self::LogValueType>;

    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }

    fn get_log_value(&self, date: NaiveDate) -> Result<Option<Self::LogValueType>> {
        let total = self
            .statistics
            .iter()
            .try_fold(chrono::Duration::zero(), |acc, s| {
                // Propagate any errors using ?
                let value = s.get_log_value(date)?;

                // Extract the duration or use zero if the value is None
                Ok::<TimeDelta, StatisticError>(
                    acc + value.map_or(chrono::Duration::zero(), |v| v.get_total_duration()),
                )
            })?;

        // Return the total duration wrapped in Result and Some
        Ok(Some(total.into()))
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

impl IntegerStatistic for SummingStatistic {
    fn value_into_i64(&self, value: Self::LogValueType) -> i64 {
        value.into()
    }
}
