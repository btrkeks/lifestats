use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use crate::services::DailyAchievementEvaluator;
use crate::time::Clock;
use chrono::Days;
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use super::query::GetStreakQuery;

/// Handler for GetStreakQuery
///
/// Refactored to use DailyAchievementEvaluator for efficient bulk evaluation.
/// - Uses Clock abstraction for deterministic testing
/// - Delegates to evaluator for bulk data fetching (no N+1 queries)
/// - Iterates newest → oldest, counting consecutive achieved days
/// - Skips days with no goal (preserving original semantics)
pub struct GetStreakQueryHandler<R, C, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    C: Clock + Send + Sync + 'static,
    E: DailyAchievementEvaluator + Send + Sync + 'static,
{
    repository: Arc<R>,
    clock: Arc<C>,
    evaluator: Arc<E>,
}

impl<R, C, E> GetStreakQueryHandler<R, C, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    C: Clock + Send + Sync + 'static,
    E: DailyAchievementEvaluator + Send + Sync + 'static,
{
    pub fn new(repository: Arc<R>, clock: Arc<C>, evaluator: Arc<E>) -> Self {
        Self {
            repository,
            clock,
            evaluator,
        }
    }
}

impl<R, C, E> QueryHandler<GetStreakQuery> for GetStreakQueryHandler<R, C, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    C: Clock + Send + Sync + 'static,
    E: DailyAchievementEvaluator + Send + Sync + 'static,
{
    fn handle(&self, query: GetStreakQuery) -> ApplicationResult<u32> {
        const MAX_LOOKBACK_DAYS: u32 = 90;

        // Get today and calculate the lookback range
        let today = self.clock.today();
        let start = today
            .checked_sub_days(Days::new((MAX_LOOKBACK_DAYS - 1) as u64))
            .unwrap_or(today);
        let range = DateRange::new(start, today)?;

        // Fetch statistic metadata
        let stat = self.repository.find_metadata_by_id(query.statistic_id)?;

        // Bulk evaluate all days in the range
        let daily_evals = self.evaluator.eval_range(&stat, range)?;

        // Count streak: iterate newest → oldest, stop at first miss
        let mut streak: u32 = 0;
        for day_eval in daily_evals.iter().rev() {
            // Iterate in reverse (newest first)
            if !day_eval.has_goal {
                // Skip days with no goal (preserve original semantics)
                continue;
            }

            if day_eval.achieved {
                streak += 1;
            } else {
                // First missed day - break the streak
                break;
            }
        }

        Ok(streak)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::any_statistic::AnyStatistic;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

    use crate::services::{DayEval, MockDailyAchievementEvaluator};
    use crate::time::Clock;

    struct StubClock {
        fixed_date: NaiveDate,
    }

    impl Clock for StubClock {
        fn now_utc(&self) -> chrono::DateTime<chrono::Utc> {
            self.fixed_date.and_hms_opt(12, 0, 0).unwrap().and_utc()
        }

        fn today(&self) -> NaiveDate {
            self.fixed_date
        }
    }

    #[test]
    fn test_get_streak_with_no_goals() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
        let stat_id = StatisticId(1);

        let mut repo = MockStatisticRepository::new();
        let stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(stat_id, "test".into(), "test_stat".into()),
            None,
            today,
        ));

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        let mut evaluator = MockDailyAchievementEvaluator::new();
        evaluator.expect_eval_range().times(1).returning(|_, _| {
            // Return 3 days with no goals
            Ok(vec![
                DayEval {
                    date: NaiveDate::from_ymd_opt(2025, 1, 8).unwrap(),
                    has_goal: false,
                    achieved: false,
                },
                DayEval {
                    date: NaiveDate::from_ymd_opt(2025, 1, 9).unwrap(),
                    has_goal: false,
                    achieved: false,
                },
                DayEval {
                    date: NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
                    has_goal: false,
                    achieved: false,
                },
            ])
        });

        let clock = Arc::new(StubClock { fixed_date: today });
        let handler = GetStreakQueryHandler::new(Arc::new(repo), clock, Arc::new(evaluator));

        let result = handler.handle(GetStreakQuery {
            statistic_id: stat_id,
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // No goals = 0 streak
    }

    #[test]
    fn test_get_streak_all_achieved() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
        let stat_id = StatisticId(1);

        let mut repo = MockStatisticRepository::new();
        let stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(stat_id, "test".into(), "test_stat".into()),
            None,
            today,
        ));

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        let mut evaluator = MockDailyAchievementEvaluator::new();
        evaluator.expect_eval_range().times(1).returning(|_, _| {
            Ok(vec![
                DayEval {
                    date: NaiveDate::from_ymd_opt(2025, 1, 8).unwrap(),
                    has_goal: true,
                    achieved: true,
                },
                DayEval {
                    date: NaiveDate::from_ymd_opt(2025, 1, 9).unwrap(),
                    has_goal: true,
                    achieved: true,
                },
                DayEval {
                    date: NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
                    has_goal: true,
                    achieved: true,
                },
            ])
        });

        let clock = Arc::new(StubClock { fixed_date: today });
        let handler = GetStreakQueryHandler::new(Arc::new(repo), clock, Arc::new(evaluator));

        let result = handler.handle(GetStreakQuery {
            statistic_id: stat_id,
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 3); // All 3 days achieved
    }

    #[test]
    fn test_get_streak_breaks_on_first_miss() {
        let today = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
        let stat_id = StatisticId(1);

        let mut repo = MockStatisticRepository::new();
        let stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(stat_id, "test".into(), "test_stat".into()),
            None,
            today,
        ));

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        let mut evaluator = MockDailyAchievementEvaluator::new();
        evaluator.expect_eval_range().times(1).returning(|_, _| {
            Ok(vec![
                DayEval {
                    date: NaiveDate::from_ymd_opt(2025, 1, 8).unwrap(),
                    has_goal: true,
                    achieved: true,
                },
                DayEval {
                    date: NaiveDate::from_ymd_opt(2025, 1, 9).unwrap(),
                    has_goal: true,
                    achieved: false, // Missed day
                },
                DayEval {
                    date: NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
                    has_goal: true,
                    achieved: true,
                },
            ])
        });

        let clock = Arc::new(StubClock { fixed_date: today });
        let handler = GetStreakQueryHandler::new(Arc::new(repo), clock, Arc::new(evaluator));

        let result = handler.handle(GetStreakQuery {
            statistic_id: stat_id,
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1); // Only most recent day (Jan 10) counts
    }
}
