use crate::statistic::value_objects::update_policies::update_policy_trait::UpdatePolicy;
use std::ops::Add;
use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, Copy, EnumString, Display, Default, Eq, PartialEq)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum AmountUpdatePolicy {
    #[default]
    Sum,
    Overwrite,
    Ignore,
}

impl AmountUpdatePolicy {
    /// Integer logging rejects overflow instead of losing an aggregate contribution.
    pub fn apply_checked_i64(self, current: Option<i64>, new: i64) -> Option<(i64, bool)> {
        match (self, current) {
            (Self::Sum, Some(current)) => current.checked_add(new).map(|value| (value, true)),
            _ => Some(self.apply(current, new)),
        }
    }
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

#[cfg(test)]
mod checked_tests {
    use super::*;
    #[test]
    fn checked_integer_policy_preserves_existing_semantics() {
        assert_eq!(
            AmountUpdatePolicy::Sum.apply_checked_i64(Some(10), 5),
            Some((15, true))
        );
        assert_eq!(
            AmountUpdatePolicy::Sum.apply_checked_i64(Some(i64::MAX), 1),
            None
        );
        assert_eq!(
            AmountUpdatePolicy::Overwrite.apply_checked_i64(Some(10), 4),
            Some((4, true))
        );
        assert_eq!(
            AmountUpdatePolicy::Overwrite.apply_checked_i64(Some(4), 4),
            Some((4, false))
        );
        assert_eq!(
            AmountUpdatePolicy::Ignore.apply_checked_i64(Some(10), 4),
            Some((10, false))
        );
        for policy in [
            AmountUpdatePolicy::Sum,
            AmountUpdatePolicy::Overwrite,
            AmountUpdatePolicy::Ignore,
        ] {
            assert_eq!(policy.apply_checked_i64(None, -4), Some((-4, true)));
        }
    }
}
