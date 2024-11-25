use serde::{Deserialize, Serialize};
use crate::objects::goal::Goal;
use crate::objects::my_duration::MyDuration;
use crate::objects::update_policies::amount::AmountUpdatePolicy;
use crate::statistic::{StatisticDefinition};
use crate::type_definitions::integer_amount::IntegerAmountGoal;

pub type TimeAmountStatistic = StatisticDefinition<MyDuration, MyDuration, AmountUpdatePolicy, TimeAmountGoal>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeAmountGoal {
    integer_amount_goal: IntegerAmountGoal
}

impl TimeAmountGoal {
    pub fn new(target: MyDuration, is_good: bool) -> Self {
        Self {
            integer_amount_goal: IntegerAmountGoal::new(target.secs() as i64, is_good),
        }
    }
}

impl Goal<MyDuration> for TimeAmountGoal {
    fn get_percentage_achieved(&self, value: &MyDuration) -> f64 {
        self.integer_amount_goal.get_percentage_achieved(&(value.secs() as i64))
    }
}
