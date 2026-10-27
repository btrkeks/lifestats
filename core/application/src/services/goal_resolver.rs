use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Days, NaiveDate};
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::statistic::any_statistic::{AnyGoal, AnyStatistic};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::value_objects::goals::goal_query_result::GoalQueryResult;

use crate::error::ApplicationResult;

/// Serializes the concrete goal wrapped by an `AnyGoal`.
///
/// This is the JSON shape persisted by `SetDefaultGoalCommandHandler` and returned by
/// `StatisticRepository::find_default_goal` (e.g. `{"target_amount": 10, "is_at_least": true}`).
/// Do not `serde_json::to_value(&any_goal)` directly: `AnyGoal` is externally tagged and
/// would produce `{"IntegerAmount": {...}}`.
pub fn any_goal_to_json(goal: &AnyGoal) -> serde_json::Result<serde_json::Value> {
    match goal {
        AnyGoal::Habit(g) => serde_json::to_value(g),
        AnyGoal::IntegerAmount(g) => serde_json::to_value(g),
        AnyGoal::FinishedInTime(g) => serde_json::to_value(g),
        AnyGoal::TimeSpans(g) => serde_json::to_value(g),
        AnyGoal::SecondsAmount(g) => serde_json::to_value(g),
    }
}

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
///   - Return the default goal that applied on that date (from the default goal
///     timeline), or None if none did
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

        // The default goal over time (each day is judged by the goal that applied then)
        let default_goals = self.repository.find_default_goal_timeline(stat_id)?;

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
                Some(GoalQueryResult::NotSet) | None => default_goals.goal_on(current).cloned(),
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
    use lifestats_core::statistic::value_objects::goals::default_goal_timeline::{
        DefaultGoalSegment, DefaultGoalTimeline,
    };
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use serde_json::json;

    /// Mocks the stored timeline as the statistic's single default goal segment.
    fn expect_timeline_of(repo: &mut MockStatisticRepository, stat: &AnyStatistic) {
        let goal = stat
            .get_default_goal()
            .map(|goal| any_goal_to_json(&goal).unwrap());
        let timeline = DefaultGoalTimeline::new(vec![DefaultGoalSegment::new(
            stat.get_default_goal_valid_since(),
            goal,
        )]);
        repo.expect_find_default_goal_timeline()
            .returning(move |_| Ok(timeline.clone()));
    }

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

        expect_timeline_of(&mut mock_repo, &stat);
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

        expect_timeline_of(&mut mock_repo, &stat);
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

        expect_timeline_of(&mut mock_repo, &stat);
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

        expect_timeline_of(&mut mock_repo, &stat);
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

        expect_timeline_of(&mut mock_repo, &stat);
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

        expect_timeline_of(&mut mock_repo, &stat);
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

    #[test]
    fn test_resolve_uses_the_goal_that_applied_on_each_day() {
        let mut mock_repo = MockStatisticRepository::new();
        let stat = create_test_statistic(None, None);
        let date = |day| NaiveDate::from_ymd_opt(2025, 1, day).unwrap();

        mock_repo
            .expect_find_goals_in_range()
            .returning(|_, _| Ok(vec![]));
        mock_repo
            .expect_find_default_goal_timeline()
            .returning(move |_| {
                Ok(DefaultGoalTimeline::new(vec![
                    DefaultGoalSegment::new(date(2), Some(json!({"target_amount": 10}))),
                    DefaultGoalSegment::new(date(4), Some(json!({"target_amount": 20}))),
                ]))
            });

        let resolver = GoalResolver::new(Arc::new(mock_repo));
        let range = DateRange::new(date(1), date(5)).unwrap();
        let result = resolver.resolve_goals_for_range(&stat, range).unwrap();

        assert_eq!(result[&date(1)], None);
        assert_eq!(result[&date(3)], Some(json!({"target_amount": 10})));
        assert_eq!(result[&date(4)], Some(json!({"target_amount": 20})));
        assert_eq!(result[&date(5)], Some(json!({"target_amount": 20})));
    }
}
