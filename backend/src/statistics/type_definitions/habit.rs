use serde::{Deserialize, Serialize};
use crate::statistics::goal::Goal;
use crate::statistics::type_definitions::{Statistic, StatisticMetadata, StatisticTypeSpecific};
use crate::statistics::update_policy::FixedValueUpdatePolicy;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HabitStatistic {
    metadata: StatisticMetadata,
    goal: Option<HabitGoal>,
}

impl Statistic for HabitStatistic {
    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }
}

impl StatisticTypeSpecific for HabitStatistic {
    type LogValue = bool;
    type RetrieveValue = bool;
    type UpdatePolicy = FixedValueUpdatePolicy;
    type GoalType = HabitGoal;

    fn get_goal(&self) -> &Option<Self::GoalType> {
        &self.goal
    }

    // fn parse_string_to_value(&self, string: &str) -> anyhow::Result<Self::LogValue> {
    //     parse_bool(string)
    //         .with_context(|| anyhow!("Parsing {} into a bool for id {:?}", string, self.get_id()))
    // }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitGoal {
    is_good: bool,
}

impl Goal<bool> for HabitGoal {
    fn get_percentage_achieved(&self, value: &bool) -> f64 {
        if !(value ^ self.is_good) { 1.0 } else { 0.0 }
    }
}