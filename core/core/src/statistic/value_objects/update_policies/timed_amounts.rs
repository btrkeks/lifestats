use crate::statistic::value_objects::update_policies::update_policy_trait::UpdatePolicy;
use strum_macros::{Display, EnumString};

/// A timed-amounts day is only ever replaced as a whole. With a single variant the
/// default cannot ignore or append a second value.
#[derive(Debug, Clone, Copy, EnumString, Display, Default, Eq, PartialEq)]
#[strum(serialize_all = "lowercase", ascii_case_insensitive)]
pub enum TimedAmountsUpdatePolicy {
    #[default]
    Overwrite,
}

impl<T: PartialEq> UpdatePolicy<T> for TimedAmountsUpdatePolicy {
    fn apply(&self, current: Option<T>, new: T) -> (T, bool) {
        let changed = current.as_ref() != Some(&new);
        (new, changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::timed_amount_list::TimedAmountList;

    #[test]
    fn replaying_an_equal_list_is_unchanged() {
        let list: TimedAmountList =
            r#"[{"time":"13:40:00","amount":60},{"time":"08:12:00","amount":120}]"#
                .parse()
                .unwrap();
        let reordered: TimedAmountList =
            r#"[{"time":"08:12:00","amount":120},{"time":"13:40:00","amount":60}]"#
                .parse()
                .unwrap();
        let policy = TimedAmountsUpdatePolicy::default();
        assert_eq!(policy.apply(None, list.clone()), (list.clone(), true));
        assert_eq!(
            policy.apply(Some(list.clone()), reordered),
            (list.clone(), false)
        );
        let empty = TimedAmountList::default();
        assert_eq!(policy.apply(Some(list), empty.clone()), (empty, true));
    }

    #[test]
    fn only_overwrite_parses() {
        assert_eq!(
            "Overwrite".parse::<TimedAmountsUpdatePolicy>().unwrap(),
            TimedAmountsUpdatePolicy::Overwrite
        );
        assert!("append".parse::<TimedAmountsUpdatePolicy>().is_err());
        assert!("ignore".parse::<TimedAmountsUpdatePolicy>().is_err());
    }
}
