use crate::shared::ids::StatisticId;
use crate::shared::seconds::Seconds;
use crate::statistic::traits::base_statistic::Statistic;
use crate::statistic::traits::integer_statistic::IntegerStatistic;
use crate::statistic::value_objects::goals::amount_goal::AmountGoal;
use crate::statistic::value_objects::statistic_metadata::StatisticMetadata;
use chrono::NaiveDate;

/// This statistic takes a list of time spans statistics and sum the total time
#[derive(Clone)]
pub struct SummingStatistic {
    metadata: StatisticMetadata,
    /// IDs of source TimeSpans statistics to sum
    source_statistic_ids: Vec<StatisticId>,
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
    fn value_as_i64(&self, value: &Self::LogValueType) -> i64 {
        (*value).into()
    }
}

impl SummingStatistic {
    pub fn new(
        metadata: StatisticMetadata,
        source_statistic_ids: Vec<StatisticId>,
        default_goal: Option<<Self as Statistic>::GoalType>,
        default_goal_start: NaiveDate,
    ) -> Self {
        Self {
            metadata,
            source_statistic_ids,
            default_goal,
            default_goal_start,
        }
    }

    /// Get the source statistic IDs
    pub fn source_ids(&self) -> &[StatisticId] {
        &self.source_statistic_ids
    }
}
