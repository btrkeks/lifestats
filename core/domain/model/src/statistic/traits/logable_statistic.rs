use crate::statistic::error::{Result, StatisticError};
use crate::statistic::traits::base_statistic::Statistic;
use crate::statistic::update_policies::update_policy_trait::UpdatePolicy;
use chrono::NaiveDate;
use std::str::FromStr;

pub trait LogableStatistic: Statistic
where
    Self::LogValueType: FromStr,
{
    type UpdatePolicyType: UpdatePolicy<Self::LogValueType>;

    fn log_value(
        &self,
        date: NaiveDate,
        value: Self::LogValueType,
        update_policy: Self::UpdatePolicyType,
    ) -> Result<()>;

    fn log_from_string(
        &self,
        date: NaiveDate,
        value_str: &str,
        update_policy_str: Option<&str>,
    ) -> Result<()> {
        let value = match value_str.parse::<Self::LogValueType>() {
            Ok(v) => v,
            Err(_) => return Err(StatisticError::InvalidValue(format!("Could not parse: {}", value_str))),
        };

        let update_policy = match update_policy_str {
            Some(up_str) => match up_str.parse::<Self::UpdatePolicyType>() {
                Ok(up) => up,
                Err(_) => return Err(StatisticError::UpdatePolicy(format!("Invalid update policy: {}", up_str))),
            },
            None => Self::UpdatePolicyType::default(),
        };

        self.log_value(date, value, update_policy)
    }
}
