use crate::statistic::error::{Result, StatisticError};
use crate::statistic::repositories::goal_repository::GoalRepository;
use crate::statistic::repositories::log_repository::StatisticLogRepository;
use crate::statistic::traits::base_statistic::Statistic;
use crate::statistic::traits::logable_statistic::LogableStatistic;
use crate::statistic::value_objects::goals::time_spans_goal::TimeSpansGoal;
use crate::statistic::value_objects::statistic_metadata::StatisticMetadata;
use crate::statistic::value_objects::update_policies::time_frame_list::TimeFrameListUpdatePolicy;
use crate::statistic::value_objects::update_policies::update_policy_trait::UpdatePolicy;
use crate::shared::time_frame_list::TimeFrameList;
use chrono::{NaiveDate, NaiveTime};

/// A statistic that represents a list of time spans
///
/// Examples:
/// - Time studied (e.g. [10:00 am - 11:00 am, 1 pm - 2 pm])
/// - Time book read
pub struct TimeSpansStatistic {
    // Metadata
    metadata: StatisticMetadata,

    // Logging related
    logged_values:
        Box<dyn StatisticLogRepository<LogValueType = <Self as Statistic>::LogValueType>>,

    // Goal related
    goals: Box<dyn GoalRepository<GoalType = <Self as Statistic>::GoalType>>,
    /// The current default goal
    default_goal: Option<<Self as Statistic>::GoalType>,
    /// The date from which the current default goal became active (including)
    default_goal_start: NaiveDate,
}

impl TimeSpansStatistic {
    pub fn new(
        metadata: StatisticMetadata,
        logged_values: Box<
            dyn StatisticLogRepository<LogValueType = <Self as Statistic>::LogValueType>,
        >,
        goals: Box<dyn GoalRepository<GoalType = <Self as Statistic>::GoalType>>,
        default_goal: Option<<Self as Statistic>::GoalType>,
        default_goal_start: NaiveDate,
    ) -> Self {
        Self {
            metadata,
            logged_values,
            goals,
            default_goal,
            default_goal_start,
        }
    }
}

impl Statistic for TimeSpansStatistic {
    type LogValueType = TimeFrameList;
    type GoalType = TimeSpansGoal;

    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }

    fn get_log_value(&self, date: NaiveDate) -> Result<Option<Self::LogValueType>> {
        self.logged_values
            .get(self.id(), date)
            .map_err(|e| StatisticError::Repository(e.to_string()))
    }

    fn get_goal(&self, date: NaiveDate) -> Result<Option<Self::GoalType>> {
        self.goals
            .get(self.id(), date)
            .map_err(|e| StatisticError::Repository(e.to_string()))
    }

    fn set_goal(&self, date: NaiveDate, goal: Self::GoalType) -> Result<()> {
        self.goals
            .save(self.id(), date, goal)
            .map_err(|e| StatisticError::Repository(e.to_string()))
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

impl LogableStatistic for TimeSpansStatistic {
    type UpdatePolicyType = TimeFrameListUpdatePolicy;

    fn log_value(
        &self,
        date: NaiveDate,
        log_value: Self::LogValueType,
        update_policy: Self::UpdatePolicyType,
    ) -> Result<()> {
        let current_value = self.get_log_value(date)?;
        let (new_value, changed) = update_policy.apply(current_value, log_value);

        if changed {
            self.logged_values.save(self.id(), date, new_value)?;
        }

        Ok(())
    }
}

impl TimeSpansStatistic {
    /// Returns the start time of the first recorded time span
    fn get_start_time(&self, date: NaiveDate) -> Result<Option<NaiveTime>> {
        let log_value = self.get_log_value(date)?;
        Ok(log_value.map(|v| v.start_time()).flatten())
    }
}
