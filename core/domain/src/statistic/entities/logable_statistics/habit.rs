use crate::statistic::repositories::goal_repository::GoalRepository;
use crate::statistic::repositories::log_repository::StatisticLogRepository;
use crate::statistic::traits::base_statistic::Statistic;
use crate::statistic::traits::logable_statistic::LogableStatistic;
use chrono::NaiveDate;
use crate::statistic::error::Result;
use crate::statistic::value_objects::statistic_metadata::StatisticMetadata;
use crate::statistic::value_objects::goals::habit_goal::HabitGoal;
use crate::statistic::value_objects::update_policies::fixed_value::FixedValueUpdatePolicy;
use crate::statistic::value_objects::update_policies::update_policy_trait::UpdatePolicy;

/// A statistic that represents a habit
///
/// Examples:
/// - Whether you brushed your teeth that day
/// - Whether you went to the gym that day
/// - Whether you avoided touching your phone in the morning
pub struct HabitStatistic {
    // Metadata
    metadata: StatisticMetadata,

    // Logging related
    logged_values: Box<dyn StatisticLogRepository<LogValueType=<Self as Statistic>::LogValueType>>,

    // Goal related
    goals: Box<dyn GoalRepository<GoalType=<Self as Statistic>::GoalType>>,
    /// The current default goal
    default_goal: Option<<Self as Statistic>::GoalType>,
    /// The date from which the current default goal became active (including)
    default_goal_start: NaiveDate,
}

impl Statistic for HabitStatistic {
    type LogValueType = bool;
    type GoalType = HabitGoal;

    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }

    fn get_log_value(&self, date: NaiveDate) -> Result<Option<Self::LogValueType>> {
        Ok(self.logged_values.get(self.id(), date)?)
    }

    fn get_goal(&self, date: NaiveDate) -> Result<Option<Self::GoalType>> {
        Ok(self.goals.get(self.id(), date)?)
    }

    fn set_goal(&self, date: NaiveDate, goal: Self::GoalType) -> Result<()> {
        Ok(self.goals.save(self.id(), date, goal)?)
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

impl LogableStatistic for HabitStatistic {
    type UpdatePolicyType = FixedValueUpdatePolicy;

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
