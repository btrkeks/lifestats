use serde::{Deserialize, Serialize};
use crate::statistics::goal::Goal;
use crate::statistics::type_definitions::{Statistic, StatisticMetadata, StatisticTypeSpecific};
use crate::statistics::update_policy::AmountUpdatePolicy;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IntegerAmountStatistic {
    metadata: StatisticMetadata,
    goal: Option<IntegerAmountGoal>,
    unit: String,
}

impl Statistic for IntegerAmountStatistic {
    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }
}

impl StatisticTypeSpecific for IntegerAmountStatistic {
    type LogValue = i64;
    type RetrieveValue = i64;
    type UpdatePolicy = AmountUpdatePolicy;
    type GoalType = IntegerAmountGoal;

    fn get_goal(&self) -> &Option<IntegerAmountGoal> {
        &self.goal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegerAmountGoal {
    goal: i64,
    is_good: bool,
}

impl Goal<i64> for IntegerAmountGoal {
    fn get_percentage_achieved(&self, value: &i64) -> f64 {
        if self.is_good {
            *value as f64 / self.goal as f64
        } else {
            self.goal as f64 / value.max(&1).to_owned() as f64
        }
    }
}