use std::collections::HashMap;
use std::sync::Arc;

use chrono::NaiveDate;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::value_objects::goals::goal_query_result::GoalQueryResult;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use crate::services::{DailyAchievementEvaluator, GoalResolver};

use super::query::{
    DailyAchievement, DailyAchievementsResult, DailyGoalOverrideKind, GetDailyAchievementsQuery,
};

/// Handler for GetDailyAchievementsQuery.
///
/// Responsibilities:
/// - Delegate per-day evaluation to DailyAchievementEvaluator
/// - Attach the daily override state (none / suppressed / goal), the logged value
///   and the applicable goal for each day
pub struct GetDailyAchievementsQueryHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: DailyAchievementEvaluator + Send + Sync + 'static,
{
    repository: Arc<R>,
    evaluator: Arc<E>,
}

impl<R, E> GetDailyAchievementsQueryHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: DailyAchievementEvaluator + Send + Sync + 'static,
{
    pub fn new(repository: Arc<R>, evaluator: Arc<E>) -> Self {
        Self {
            repository,
            evaluator,
        }
    }
}

impl From<&GoalQueryResult> for DailyGoalOverrideKind {
    fn from(result: &GoalQueryResult) -> Self {
        match result {
            GoalQueryResult::Goal(_) => DailyGoalOverrideKind::Goal,
            GoalQueryResult::Suppressed => DailyGoalOverrideKind::Suppressed,
            GoalQueryResult::NotSet => DailyGoalOverrideKind::None,
        }
    }
}

impl<R, E> QueryHandler<GetDailyAchievementsQuery> for GetDailyAchievementsQueryHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: DailyAchievementEvaluator + Send + Sync + 'static,
{
    fn handle(
        &self,
        query: GetDailyAchievementsQuery,
    ) -> ApplicationResult<DailyAchievementsResult> {
        let statistic = self.repository.find_metadata_by_id(query.statistic_id)?;

        let evals = self.evaluator.eval_range(&statistic, query.date_range)?;
        let mut values = self
            .evaluator
            .values_in_range(&statistic, query.date_range)?;
        let mut goals = GoalResolver::new(self.repository.clone())
            .resolve_goals_for_range(&statistic, query.date_range)?;

        let overrides: HashMap<NaiveDate, DailyGoalOverrideKind> = self
            .repository
            .find_goals_in_range(query.statistic_id, query.date_range)?
            .iter()
            .map(|dated| (dated.date, DailyGoalOverrideKind::from(&dated.value)))
            .collect();

        let days = evals
            .into_iter()
            .map(|eval| DailyAchievement {
                date: eval.date,
                has_goal: eval.has_goal,
                achieved: eval.achieved,
                percentage_achieved: eval.percentage_achieved,
                override_kind: overrides
                    .get(&eval.date)
                    .copied()
                    .unwrap_or(DailyGoalOverrideKind::None),
                value: values.remove(&eval.date),
                goal: goals.remove(&eval.date).flatten(),
            })
            .collect();

        Ok(DailyAchievementsResult {
            statistic_id: query.statistic_id,
            start_date: query.date_range.start(),
            end_date: query.date_range.end(),
            days,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifestats_core::shared::date_range::DateRange;
    use lifestats_core::shared::dated_value::DatedValue;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::shared::percentage::Percentage;
    use lifestats_core::statistic::any_statistic::AnyStatistic;
    use lifestats_core::statistic::ports::error::RepositoryError;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use serde_json::json;

    use crate::services::{DayEval, DayValue, MockDailyAchievementEvaluator};
    use lifestats_core::statistic::value_objects::goals::default_goal_timeline::DefaultGoalTimeline;

    fn date(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2025, 1, day).unwrap()
    }

    fn test_statistic(id: StatisticId) -> AnyStatistic {
        AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(id, "Test".into(), "test_stat".into()),
            None,
            date(1),
        ))
    }

    fn three_day_evals() -> Vec<DayEval> {
        vec![
            DayEval {
                date: date(1),
                has_goal: true,
                achieved: true,
                percentage_achieved: Some(Percentage::new(100)),
            },
            DayEval {
                date: date(2),
                has_goal: true,
                achieved: false,
                percentage_achieved: Some(Percentage::new(50)),
            },
            DayEval {
                date: date(3),
                has_goal: false,
                achieved: false,
                percentage_achieved: None,
            },
        ]
    }

    #[test]
    fn test_get_daily_achievements_maps_override_kind_per_day() {
        let stat_id = StatisticId(1);
        let range = DateRange::new(date(1), date(3)).unwrap();

        let mut repo = MockStatisticRepository::new();
        let stat = test_statistic(stat_id);
        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));
        // once for the override kinds, once by the goal resolver
        repo.expect_find_goals_in_range()
            .times(2)
            .returning(|_, _| {
                Ok(vec![
                    DatedValue::new(
                        date(1),
                        GoalQueryResult::Goal(json!({"target_amount": 5, "is_at_least": true})),
                    ),
                    DatedValue::new(date(3), GoalQueryResult::Suppressed),
                ])
            });

        repo.expect_find_default_goal_timeline()
            .returning(|_| Ok(DefaultGoalTimeline::default()));

        let mut evaluator = MockDailyAchievementEvaluator::new();
        evaluator
            .expect_eval_range()
            .times(1)
            .returning(|_, _| Ok(three_day_evals()));
        evaluator
            .expect_values_in_range()
            .times(1)
            .returning(|_, _| Ok(HashMap::from([(date(1), DayValue::Amount(7))])));

        let handler = GetDailyAchievementsQueryHandler::new(Arc::new(repo), Arc::new(evaluator));
        let result = handler
            .handle(GetDailyAchievementsQuery {
                statistic_id: stat_id,
                date_range: range,
            })
            .unwrap();

        assert_eq!(result.statistic_id, stat_id);
        assert_eq!(result.start_date, date(1));
        assert_eq!(result.end_date, date(3));
        assert_eq!(result.days.len(), 3);

        assert_eq!(result.days[0].override_kind, DailyGoalOverrideKind::Goal);
        assert!(result.days[0].achieved);
        assert_eq!(
            result.days[0].percentage_achieved,
            Some(Percentage::new(100))
        );

        assert_eq!(result.days[0].value, Some(DayValue::Amount(7)));
        assert_eq!(
            result.days[0].goal,
            Some(json!({"target_amount": 5, "is_at_least": true}))
        );

        assert_eq!(result.days[1].override_kind, DailyGoalOverrideKind::None);
        assert_eq!(result.days[1].value, None);
        assert!(result.days[1].has_goal);
        assert!(!result.days[1].achieved);
        assert_eq!(
            result.days[1].percentage_achieved,
            Some(Percentage::new(50))
        );

        assert_eq!(
            result.days[2].override_kind,
            DailyGoalOverrideKind::Suppressed
        );
        assert!(!result.days[2].has_goal);
        assert_eq!(result.days[2].goal, None);
        assert_eq!(result.days[2].percentage_achieved, None);
    }

    #[test]
    fn test_get_daily_achievements_no_overrides_defaults_to_none() {
        let stat_id = StatisticId(1);
        let range = DateRange::new(date(1), date(3)).unwrap();

        let mut repo = MockStatisticRepository::new();
        let stat = test_statistic(stat_id);
        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));
        // once for the override kinds, once by the goal resolver
        repo.expect_find_goals_in_range()
            .times(2)
            .returning(|_, _| Ok(vec![]));

        repo.expect_find_default_goal_timeline()
            .returning(|_| Ok(DefaultGoalTimeline::default()));

        let mut evaluator = MockDailyAchievementEvaluator::new();
        evaluator
            .expect_eval_range()
            .times(1)
            .returning(|_, _| Ok(three_day_evals()));
        evaluator
            .expect_values_in_range()
            .times(1)
            .returning(|_, _| Ok(HashMap::from([(date(1), DayValue::Amount(7))])));

        let handler = GetDailyAchievementsQueryHandler::new(Arc::new(repo), Arc::new(evaluator));
        let result = handler
            .handle(GetDailyAchievementsQuery {
                statistic_id: stat_id,
                date_range: range,
            })
            .unwrap();

        assert!(result
            .days
            .iter()
            .all(|d| d.override_kind == DailyGoalOverrideKind::None));
    }

    #[test]
    fn test_get_daily_achievements_statistic_not_found() {
        let range = DateRange::new(date(1), date(3)).unwrap();

        let mut repo = MockStatisticRepository::new();
        repo.expect_find_metadata_by_id()
            .times(1)
            .returning(|_| Err(RepositoryError::NotFound));
        let evaluator = MockDailyAchievementEvaluator::new();

        let handler = GetDailyAchievementsQueryHandler::new(Arc::new(repo), Arc::new(evaluator));
        let result = handler.handle(GetDailyAchievementsQuery {
            statistic_id: StatisticId(99),
            date_range: range,
        });

        assert!(result.is_err());
    }
}
