use crate::shared::percentage::Percentage;
use crate::shared::timed_amount_list::{TimedAmountList, MAX_DAILY_AMOUNT};
use crate::statistic::value_objects::goals::amount_goal::AmountGoal;
use crate::statistic::value_objects::goals::goal_trait::Goal;
use serde::{Deserialize, Deserializer, Serialize};

/// A cap or minimum on a timed-amounts day's total, e.g. at most 400 mg caffeine.
///
/// `Statistic::GoalType` must score the native list, so this wraps `AmountGoal<i64>`
/// and applies it to `TimedAmountList::total`. Stored JSON is AmountGoal's:
/// `{"target_amount":400,"is_at_least":false}`.
///
/// Unlike IntegerAmount, a day that was never sent scores 0%: a sync that did not
/// run must not look like a day without intake. A sent empty list (`[]`) is a
/// confirmed zero and scores like a total of 0.
///
/// A "nothing after 14:00" cutoff is deliberately not part of this goal yet; it
/// needs the logical-day boundary first (see `TimedAmountList`).
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct TimedAmountsGoal(AmountGoal<i64>);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Goal target must be between 0 and {MAX_DAILY_AMOUNT}")]
pub struct TimedAmountsGoalError;

impl TimedAmountsGoal {
    /// `0 <= target <= MAX_DAILY_AMOUNT`, which keeps AmountGoal's `* 100` in range.
    pub fn new(target_amount: i64, is_at_least: bool) -> Result<Self, TimedAmountsGoalError> {
        if !(0..=MAX_DAILY_AMOUNT).contains(&target_amount) {
            return Err(TimedAmountsGoalError);
        }
        Ok(Self(AmountGoal::new(target_amount, is_at_least)))
    }

    pub fn target_amount(&self) -> i64 {
        self.0.target_amount()
    }

    pub fn is_at_least(&self) -> bool {
        self.0.is_at_least()
    }
}

impl<'de> Deserialize<'de> for TimedAmountsGoal {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let goal = AmountGoal::<i64>::deserialize(deserializer)?;
        TimedAmountsGoal::new(goal.target_amount(), goal.is_at_least())
            .map_err(serde::de::Error::custom)
    }
}

impl Goal for TimedAmountsGoal {
    type LogValue = TimedAmountList;

    fn get_percentage_achieved(&self, value: Option<&TimedAmountList>) -> Percentage {
        match value {
            None => Percentage::new(0),
            Some(list) => self.0.get_percentage_achieved(Some(&list.total())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn list(raw: &str) -> TimedAmountList {
        raw.parse().unwrap()
    }

    fn percent(goal: &TimedAmountsGoal, value: Option<&TimedAmountList>) -> u8 {
        goal.get_percentage_achieved(value).as_num()
    }

    #[test]
    fn never_sent_misses_but_confirmed_zero_meets_a_cap() {
        let cap = TimedAmountsGoal::new(400, false).unwrap();
        assert_eq!(percent(&cap, None), 0);
        assert!(!cap.is_achieved(None));
        assert_eq!(percent(&cap, Some(&list("[]"))), 100);
        assert!(cap.is_achieved(Some(&list("[]"))));
    }

    #[test]
    fn scores_the_daily_total_like_an_amount_goal() {
        let cap = TimedAmountsGoal::new(400, false).unwrap();
        let day = list(r#"[{"time":"08:12:00","amount":120},{"time":null,"amount":60}]"#);
        assert_eq!(percent(&cap, Some(&day)), 222);
        let over = list(r#"[{"time":"08:00:00","amount":300},{"time":"12:00:00","amount":500}]"#);
        assert_eq!(percent(&cap, Some(&over)), 50);
        assert!(!cap.is_achieved(Some(&over)));

        let minimum = TimedAmountsGoal::new(400, true).unwrap();
        assert_eq!(percent(&minimum, Some(&day)), 45);
        assert_eq!(percent(&minimum, Some(&list("[]"))), 0);
    }

    #[test]
    fn extreme_targets_and_totals_do_not_overflow() {
        let huge = list(&format!(r#"[{{"time":null,"amount":{MAX_DAILY_AMOUNT}}}]"#));
        let cap = TimedAmountsGoal::new(MAX_DAILY_AMOUNT, false).unwrap();
        assert_eq!(
            percent(&cap, Some(&list(r#"[{"time":null,"amount":1}]"#))),
            255
        );
        assert_eq!(percent(&cap, Some(&huge)), 100);
        let minimum = TimedAmountsGoal::new(1, true).unwrap();
        assert_eq!(percent(&minimum, Some(&huge)), 255);
    }

    #[test]
    fn stores_amount_goal_json_and_validates_on_read() {
        let goal = TimedAmountsGoal::new(400, false).unwrap();
        let stored = serde_json::to_value(&goal).unwrap();
        assert_eq!(stored, json!({"target_amount": 400, "is_at_least": false}));
        let read: TimedAmountsGoal = serde_json::from_value(stored).unwrap();
        assert_eq!(read.target_amount(), 400);
        assert!(!read.is_at_least());

        assert!(TimedAmountsGoal::new(-1, false).is_err());
        assert!(TimedAmountsGoal::new(MAX_DAILY_AMOUNT + 1, false).is_err());
        assert!(serde_json::from_value::<TimedAmountsGoal>(
            json!({"target_amount": i64::MAX, "is_at_least": false})
        )
        .is_err());
    }
}
