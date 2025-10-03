use crate::shared::seconds::Seconds;
use crate::statistic::traits::base_statistic::Statistic;
use crate::statistic::traits::integer_statistic::IntegerStatistic;
use crate::statistic::types::logable_statistics::point_in_time::PointInTimeStatistic;
use crate::statistic::value_objects::goals::amount_goal::AmountGoal;
use crate::statistic::value_objects::statistic_metadata::StatisticMetadata;
use chrono::NaiveDate;

#[derive(Clone)]
pub struct TimeDifferenceStatistic {
    metadata: StatisticMetadata,

    statistic0: PointInTimeStatistic,
    statistic1: PointInTimeStatistic,
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

impl TimeDifferenceStatistic {
    pub fn new(
        metadata: StatisticMetadata,
        statistic0: PointInTimeStatistic,
        statistic1: PointInTimeStatistic,
        default_goal: Option<<Self as Statistic>::GoalType>,
        default_goal_start: NaiveDate,
    ) -> Self {
        Self {
            metadata,
            statistic0,
            statistic1,
            default_goal,
            default_goal_start,
        }
    }
}
