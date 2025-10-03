use crate::shared::time_frame_list::TimeFrameList;
use crate::statistic::traits::base_statistic::Statistic;
use crate::statistic::traits::logable_statistic::LogableStatistic;
use crate::statistic::value_objects::goals::time_spans_goal::TimeSpansGoal;
use crate::statistic::value_objects::statistic_metadata::StatisticMetadata;
use crate::statistic::value_objects::update_policies::time_frame_list::TimeFrameListUpdatePolicy;
use crate::statistic::value_objects::update_policies::update_policy_trait::UpdatePolicy;
use chrono::NaiveDate;

/// A statistic that represents a list of time spans
///
/// Examples:
/// - Time studied (e.g. [10:00 am - 11:00 am, 1 pm - 2 pm])
/// - Time book read
pub struct TimeSpansStatistic {
    // Metadata
    metadata: StatisticMetadata,
    /// The current default goal
    default_goal: Option<<Self as Statistic>::GoalType>,
    /// The date from which the current default goal became active (including)
    default_goal_start: NaiveDate,
}

impl TimeSpansStatistic {
    pub fn new(
        metadata: StatisticMetadata,
        default_goal: Option<<Self as Statistic>::GoalType>,
        default_goal_start: NaiveDate,
    ) -> Self {
        Self {
            metadata,
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

    fn calculate_new_log_value(
        &self,
        current_value: Option<Self::LogValueType>,
        new_value: Self::LogValueType,
        update_policy: Self::UpdatePolicyType,
    ) -> (Self::LogValueType, bool) {
        update_policy.apply(current_value, new_value)
    }
}

impl TimeSpansStatistic {
    pub fn calculate_new_log_value(
        &self,
        current_value: Option<TimeFrameList>,
        new_value: TimeFrameList,
        update_policy: TimeFrameListUpdatePolicy,
    ) -> (TimeFrameList, bool) {
        <Self as LogableStatistic>::calculate_new_log_value(
            self,
            current_value,
            new_value,
            update_policy,
        )
    }
}
