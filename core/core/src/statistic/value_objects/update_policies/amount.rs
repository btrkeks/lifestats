use crate::statistic::value_objects::update_policies::update_policy_trait::UpdatePolicy;
use std::ops::Add;
use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, Copy, EnumString, Display, Default, Eq, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum AmountUpdatePolicy {
    #[default]
    Sum,
    Overwrite,
    Ignore,
}

impl<T: Add<Output = T> + Copy + Default + PartialEq> UpdatePolicy<T> for AmountUpdatePolicy {
    fn apply(&self, current: Option<T>, new: T) -> (T, bool) {
        match (self, current) {
            // Always mark a sum operation as changed as there is no guarantee that
            // LogValueType::default() corresponds to the 0 element
            (AmountUpdatePolicy::Sum, Some(value)) => (value + new, true),
            (AmountUpdatePolicy::Sum, None) => (new, true),
            (AmountUpdatePolicy::Overwrite, Some(value)) if value == new => (value, false),
            (AmountUpdatePolicy::Overwrite, _) => (new, true),
            (AmountUpdatePolicy::Ignore, Some(value)) => (value, false),
            (AmountUpdatePolicy::Ignore, None) => (new, true),
        }
    }
}
