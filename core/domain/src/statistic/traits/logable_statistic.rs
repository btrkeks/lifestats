use crate::statistic::error::{DomainError, DomainResult, Result};
use crate::statistic::traits::base_statistic::Statistic;
use crate::statistic::value_objects::update_policies::update_policy_trait::UpdatePolicy;
use std::str::FromStr;

pub trait LogableStatistic: Statistic
where
    Self::LogValueType: FromStr,
{
    type UpdatePolicyType: UpdatePolicy<Self::LogValueType>;

    fn calculate_new_log_value(
        &self,
        current_value: Option<Self::LogValueType>,
        new_value: Self::LogValueType,
        update_policy: Self::UpdatePolicyType,
    ) -> (Self::LogValueType, bool);

    fn parse_log_value(&self, value_str: &str) -> DomainResult<Self::LogValueType> {
        value_str
            .parse::<Self::LogValueType>()
            .map_err(|_| DomainError::ParsingError(format!("Could not parse: {}", value_str)))
    }

    fn parse_update_policy(
        &self,
        update_policy_str: Option<&str>,
    ) -> DomainResult<Self::UpdatePolicyType> {
        match update_policy_str {
            Some(up_str) => up_str.parse::<Self::UpdatePolicyType>().map_err(|_| {
                DomainError::UpdatePolicyError(format!("Invalid update policy: {}", up_str))
            }),
            None => Ok(Self::UpdatePolicyType::default()),
        }
    }

    fn calculate_new_log_value_from_str(
        &self,
        current_value: Option<Self::LogValueType>,
        value_str: &str,
        update_policy_str: Option<&str>,
    ) -> Result<(Self::LogValueType, bool)> {
        let value = self.parse_log_value(value_str)?;
        let update_policy = self.parse_update_policy(update_policy_str)?;
        Ok(self.calculate_new_log_value(current_value, value, update_policy))
    }
}
