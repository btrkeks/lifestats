use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Days, NaiveDate};
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::value_objects::goals::goal_query_result::GoalQueryResult;

use crate::error::ApplicationResult;

/// Service for resolving goals across date ranges.
///
/// This service encapsulates the logic for combining default goals with daily overrides
/// and suppressions. Consumers get a simple HashMap of dates to optional goal values,
/// without needing to understand the underlying complexity of:
/// - Default goals and their validity dates
/// - Daily goal overrides
/// - Goal suppression
///
/// # Resolution Logic
///
/// For each date in the range:
/// - If a daily override exists with a concrete goal → return Some(goal)
/// - If a daily override is suppressed → return None (even if default goal exists)
/// - If no daily override exists (NotSet):
///   - If date >= default_goal_valid_since → return default goal (if set)
///   - Otherwise → return None
pub struct GoalResolver<R: StatisticRepository> {
    repository: Arc<R>,
}

impl<R: StatisticRepository> GoalResolver<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Resolves goals for each date in the given range.
    ///
    /// Returns a HashMap where:
    /// - Key: Date within the range
    /// - Value: Some(goal) if a goal applies, None if no goal applies
    ///
    /// Dates without any goal (neither default nor override) are still included with None value.
    pub fn resolve_goals_for_range(
        &self,
        statistic: &AnyStatistic,
        range: DateRange,
    ) -> ApplicationResult<HashMap<NaiveDate, Option<serde_json::Value>>> {
        let stat_id = statistic.get_metadata().id();

        // Fetch all daily goal overrides for the range
        let daily_goals = self.repository.find_goals_in_range(stat_id, range)?;

        // Build a map of dates to their GoalQueryResult
        let mut daily_goal_map: HashMap<NaiveDate, GoalQueryResult> =
            HashMap::with_capacity(daily_goals.len());
        for dated_goal in daily_goals {
            daily_goal_map.insert(dated_goal.date, dated_goal.value);
        }

        // Get default goal information from the statistic
        let default_goal_valid_since = statistic.get_default_goal_valid_since();
        let default_goal_json = statistic.get_default_goal().and_then(|any_goal| {
            use lifestats_core::statistic::any_statistic::AnyGoal;
            match any_goal {
                AnyGoal::Habit(g) => serde_json::to_value(g).ok(),
                AnyGoal::IntegerAmount(g) => serde_json::to_value(g).ok(),
                AnyGoal::FinishedInTime(g) => serde_json::to_value(g).ok(),
                AnyGoal::TimeSpans(g) => serde_json::to_value(g).ok(),
                AnyGoal::SecondsAmount(g) => serde_json::to_value(g).ok(),
            }
        });

        // Resolve goal for each date in the range
        let mut result = HashMap::new();
        let start = range.start();
        let end = range.end();

        let num_days = (end.signed_duration_since(start).num_days() + 1) as u64;
        let mut current = start;

        for _ in 0..num_days {
            let goal_for_date = match daily_goal_map.get(&current) {
                // Case 1: Daily override with a concrete goal
                Some(GoalQueryResult::Goal(goal)) => Some(goal.clone()),

                // Case 2: Daily override is suppressed - don't use default goal
                Some(GoalQueryResult::Suppressed) => None,

                // Case 3: No daily override - check if default goal applies
                Some(GoalQueryResult::NotSet) | None => {
                    if current >= default_goal_valid_since {
                        default_goal_json.clone()
                    } else {
                        None
                    }
                }
            };

            result.insert(current, goal_for_date);

            current = current
                .checked_add_days(Days::new(1))
                .expect("Date overflow in range iteration");
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_core::shared::dated_value::DatedValue;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::any_statistic::AnyStatistic;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::value_objects::goals::amount_goal::AmountGoal;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use serde_json::json;

    fn create_test_statistic(
        default_goal: Option<AmountGoal<i64>>,
        default_goal_valid_since: Option<NaiveDate>,
    ) -> AnyStatistic {
        let metadata = StatisticMetadata::new(
            StatisticId::from(1),
            "test_category".to_string(),
            "test_stat".to_string(),
        );
        let valid_since =
            default_goal_valid_since.unwrap_or(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap());
        AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            metadata,
            default_goal,
            valid_since,
        ))
    }

    #[test]
    fn test_resolve_with_no_default_no_overrides() {
        let mut mock_repo = MockStatisticRepository::new();
        let stat = create_test_statistic(None, None);

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
        )
        .unwrap();

        mock_repo
            .expect_find_goals_in_range()
            .returning(|_, _| Ok(vec![]));

        let resolver = GoalResolver::new(Arc::new(mock_repo));
        let result = resolver.resolve_goals_for_range(&stat, range).unwrap();

        // All dates should have None
        assert_eq!(result.len(), 3);
        assert_eq!(
            result.get(&NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            Some(&None)
        );
        assert_eq!(
            result.get(&NaiveDate::from_ymd_opt(2025, 1, 2).unwrap()),
            Some(&None)
        );
        assert_eq!(
            result.get(&NaiveDate::from_ymd_opt(2025, 1, 3).unwrap()),
            Some(&None)
        );
    }

    #[test]
    fn test_resolve_with_default_goal_no_overrides() {
        let mut mock_repo = MockStatisticRepository::new();
        let default_goal = AmountGoal::new(100, true);
        let stat = create_test_statistic(
            Some(default_goal),
            Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        );

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
        )
        .unwrap();

        mock_repo
            .expect_find_goals_in_range()
            .returning(|_, _| Ok(vec![]));

        let resolver = GoalResolver::new(Arc::new(mock_repo));
        let result = resolver.resolve_goals_for_range(&stat, range).unwrap();

        // All dates should have the default goal
        assert_eq!(result.len(), 3);

        for date in [
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
        ] {
            let goal_opt = result
                .get(&date)
                .expect(&format!("Missing date {} in result", date));
            let goal = goal_opt
                .as_ref()
                .expect(&format!("Goal is None for date {}", date));
            assert_eq!(goal.get("target_amount").unwrap(), 100);
        }
    }

    #[test]
    fn test_resolve_with_daily_override() {
        let mut mock_repo = MockStatisticRepository::new();
        let default_goal = AmountGoal::new(100, true);
        let stat = create_test_statistic(
            Some(default_goal),
            Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        );

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
        )
        .unwrap();

        // Return a daily override for 2025-1-2
        mock_repo.expect_find_goals_in_range().returning(|_, _| {
            Ok(vec![DatedValue::new(
                NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                GoalQueryResult::Goal(json!({"target_amount": 200, "is_at_least": true})),
            )])
        });

        let resolver = GoalResolver::new(Arc::new(mock_repo));
        let result = resolver.resolve_goals_for_range(&stat, range).unwrap();

        assert_eq!(result.len(), 3);

        // 2025-1-1: default goal
        let goal_1 = result
            .get(&NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
            .unwrap()
            .as_ref()
            .unwrap();
        assert_eq!(goal_1.get("target_amount").unwrap(), 100);

        // 2025-1-2: overridden goal
        let goal_2 = result
            .get(&NaiveDate::from_ymd_opt(2025, 1, 2).unwrap())
            .unwrap()
            .as_ref()
            .unwrap();
        assert_eq!(goal_2.get("target_amount").unwrap(), 200);

        // 2025-1-3: default goal
        let goal_3 = result
            .get(&NaiveDate::from_ymd_opt(2025, 1, 3).unwrap())
            .unwrap()
            .as_ref()
            .unwrap();
        assert_eq!(goal_3.get("target_amount").unwrap(), 100);
    }

    #[test]
    fn test_resolve_with_suppressed_goal() {
        let mut mock_repo = MockStatisticRepository::new();
        let default_goal = AmountGoal::new(100, true);
        let stat = create_test_statistic(
            Some(default_goal),
            Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
        );

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
        )
        .unwrap();

        // Return a suppressed goal for 2025-1-2
        mock_repo.expect_find_goals_in_range().returning(|_, _| {
            Ok(vec![DatedValue::new(
                NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                GoalQueryResult::Suppressed,
            )])
        });

        let resolver = GoalResolver::new(Arc::new(mock_repo));
        let result = resolver.resolve_goals_for_range(&stat, range).unwrap();

        assert_eq!(result.len(), 3);

        // 2025-1-1: default goal
        assert!(result
            .get(&NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
            .unwrap()
            .is_some());

        // 2025-1-2: suppressed - should be None even though default exists
        assert_eq!(
            result.get(&NaiveDate::from_ymd_opt(2025, 1, 2).unwrap()),
            Some(&None)
        );

        // 2025-1-3: default goal
        assert!(result
            .get(&NaiveDate::from_ymd_opt(2025, 1, 3).unwrap())
            .unwrap()
            .is_some());
    }

    #[test]
    fn test_resolve_before_default_goal_valid_since() {
        let mut mock_repo = MockStatisticRepository::new();
        let default_goal = AmountGoal::new(100, true);
        let stat = create_test_statistic(
            Some(default_goal),
            Some(NaiveDate::from_ymd_opt(2025, 1, 2).unwrap()), // Valid from 2025-1-2
        );

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
        )
        .unwrap();

        mock_repo
            .expect_find_goals_in_range()
            .returning(|_, _| Ok(vec![]));

        let resolver = GoalResolver::new(Arc::new(mock_repo));
        let result = resolver.resolve_goals_for_range(&stat, range).unwrap();

        assert_eq!(result.len(), 3);

        // 2025-1-1: before valid_since, should be None
        assert_eq!(
            result.get(&NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            Some(&None)
        );

        // 2025-1-2: on or after valid_since, should have default
        assert!(result
            .get(&NaiveDate::from_ymd_opt(2025, 1, 2).unwrap())
            .unwrap()
            .is_some());

        // 2025-1-3: after valid_since, should have default
        assert!(result
            .get(&NaiveDate::from_ymd_opt(2025, 1, 3).unwrap())
            .unwrap()
            .is_some());
    }

    #[test]
    fn test_resolve_no_default_with_override() {
        let mut mock_repo = MockStatisticRepository::new();
        let stat = create_test_statistic(None, None);

        let range = DateRange::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
        )
        .unwrap();

        // Return a daily override for 2025-1-2 (no default goal exists)
        mock_repo.expect_find_goals_in_range().returning(|_, _| {
            Ok(vec![DatedValue::new(
                NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                GoalQueryResult::Goal(json!({"target_amount": 200, "is_at_least": true})),
            )])
        });

        let resolver = GoalResolver::new(Arc::new(mock_repo));
        let result = resolver.resolve_goals_for_range(&stat, range).unwrap();

        assert_eq!(result.len(), 3);

        // 2025-1-1: no goal
        assert_eq!(
            result.get(&NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            Some(&None)
        );

        // 2025-1-2: overridden goal
        let goal_2 = result
            .get(&NaiveDate::from_ymd_opt(2025, 1, 2).unwrap())
            .unwrap()
            .as_ref()
            .unwrap();
        assert_eq!(goal_2.get("target_amount").unwrap(), 200);

        // 2025-1-3: no goal
        assert_eq!(
            result.get(&NaiveDate::from_ymd_opt(2025, 1, 3).unwrap()),
            Some(&None)
        );
    }
}
