use crate::statistic::value_objects::update_policies::update_policy_trait::UpdatePolicy;
use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, Copy, EnumString, Display, Default)]
#[strum(serialize_all = "lowercase")]
pub enum FixedValueUpdatePolicy {
    #[default]
    Ignore,
    Overwrite,
}

impl<T: std::cmp::PartialEq> UpdatePolicy<T> for FixedValueUpdatePolicy {
    fn apply(
        &self,
        existing: Option<T>,
        new: T,
    ) -> (T, bool) {
        match (self, existing) {
            (FixedValueUpdatePolicy::Overwrite, Some(current)) => {
                if current == new {
                    (new, false)
                }
                else {
                    (new, true)
                }
            },
            (FixedValueUpdatePolicy::Overwrite, _) => (new, true),
            (FixedValueUpdatePolicy::Ignore, Some(current)) => (current, false),
            (FixedValueUpdatePolicy::Ignore, _) => (new, true),
        }
    }
}
