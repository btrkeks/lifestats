use std::ops::Add;
use strum_macros::{Display, EnumString};
use crate::objects::update_policies::UpdatePolicy;

#[derive(Debug, Clone, Copy, EnumString, Display, Default, Eq, PartialEq)]
#[strum(serialize_all = "lowercase")]
pub enum AmountUpdatePolicy {
    #[default]
    Sum,
    Overwrite,
    Ignore,
}

impl<T> UpdatePolicy<T, T> for AmountUpdatePolicy
where
    T: Add<Output=T> + Clone + Default,
{
    fn apply(&self, existing: Option<T>, new: T) -> T {
        match self {
            AmountUpdatePolicy::Sum => existing.unwrap_or_default() + new,
            AmountUpdatePolicy::Overwrite => new,
            AmountUpdatePolicy::Ignore => existing.unwrap_or(new),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_default() {
        assert_eq!(AmountUpdatePolicy::default(), AmountUpdatePolicy::Sum);
    }

    #[test]
    fn test_from_string() {
        assert_eq!(
            AmountUpdatePolicy::from_str("sum").unwrap(),
            AmountUpdatePolicy::Sum
        );
        assert_eq!(
            AmountUpdatePolicy::from_str("overwrite").unwrap(),
            AmountUpdatePolicy::Overwrite
        );
        assert_eq!(
            AmountUpdatePolicy::from_str("ignore").unwrap(),
            AmountUpdatePolicy::Ignore
        );
        assert!(AmountUpdatePolicy::from_str("invalid").is_err());
    }

    #[test]
    fn test_display() {
        assert_eq!(AmountUpdatePolicy::Sum.to_string(), "sum");
        assert_eq!(AmountUpdatePolicy::Overwrite.to_string(), "overwrite");
        assert_eq!(AmountUpdatePolicy::Ignore.to_string(), "ignore");
    }

    #[test]
    fn test_sum_policy_with_integers() {
        let policy = AmountUpdatePolicy::Sum;

        // Test with Some existing value
        assert_eq!(policy.apply(Some(5), 3), 8);
        assert_eq!(policy.apply(Some(-5), 3), -2);
        assert_eq!(policy.apply(Some(0), 3), 3);

        // Test with None existing value
        assert_eq!(policy.apply(None, 3), 3);
    }

    #[test]
    fn test_overwrite_policy_with_integers() {
        let policy = AmountUpdatePolicy::Overwrite;

        // Test with Some existing value
        assert_eq!(policy.apply(Some(5), 3), 3);
        assert_eq!(policy.apply(Some(-5), 3), 3);
        assert_eq!(policy.apply(Some(0), 3), 3);

        // Test with None existing value
        assert_eq!(policy.apply(None, 3), 3);
    }

    #[test]
    fn test_ignore_policy_with_integers() {
        let policy = AmountUpdatePolicy::Ignore;

        // Test with Some existing value
        assert_eq!(policy.apply(Some(5), 3), 5);
        assert_eq!(policy.apply(Some(-5), 3), -5);
        assert_eq!(policy.apply(Some(0), 3), 0);

        // Test with None existing value
        assert_eq!(policy.apply(None, 3), 3);
    }

    #[test]
    fn test_with_floating_point() {
        let sum_policy = AmountUpdatePolicy::Sum;
        let overwrite_policy = AmountUpdatePolicy::Overwrite;
        let ignore_policy = AmountUpdatePolicy::Ignore;

        // Test Sum policy
        assert_eq!(sum_policy.apply(Some(5.5), 3.2), 8.7);
        assert_eq!(sum_policy.apply(None, 3.2), 3.2);

        // Test Overwrite policy
        assert_eq!(overwrite_policy.apply(Some(5.5), 3.2), 3.2);
        assert_eq!(overwrite_policy.apply(None, 3.2), 3.2);

        // Test Ignore policy
        assert_eq!(ignore_policy.apply(Some(5.5), 3.2), 5.5);
        assert_eq!(ignore_policy.apply(None, 3.2), 3.2);
    }

    #[test]
    fn test_with_custom_type() {
        #[derive(Debug, Clone, PartialEq)]
        struct Amount(i32);

        impl Default for Amount {
            fn default() -> Self {
                Amount(0)
            }
        }

        impl Add for Amount {
            type Output = Self;

            fn add(self, other: Self) -> Self {
                Amount(self.0 + other.0)
            }
        }

        let sum_policy = AmountUpdatePolicy::Sum;
        let overwrite_policy = AmountUpdatePolicy::Overwrite;
        let ignore_policy = AmountUpdatePolicy::Ignore;

        // Test Sum policy
        assert_eq!(
            sum_policy.apply(Some(Amount(5)), Amount(3)),
            Amount(8)
        );
        assert_eq!(
            sum_policy.apply(None, Amount(3)),
            Amount(3)
        );

        // Test Overwrite policy
        assert_eq!(
            overwrite_policy.apply(Some(Amount(5)), Amount(3)),
            Amount(3)
        );
        assert_eq!(
            overwrite_policy.apply(None, Amount(3)),
            Amount(3)
        );

        // Test Ignore policy
        assert_eq!(
            ignore_policy.apply(Some(Amount(5)), Amount(3)),
            Amount(5)
        );
        assert_eq!(
            ignore_policy.apply(None, Amount(3)),
            Amount(3)
        );
    }
}