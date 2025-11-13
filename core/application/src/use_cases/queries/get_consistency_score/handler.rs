use std::sync::Arc;

use chrono::Days;
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use crate::services::DailyAchievementEvaluator;
use crate::time::Clock;

use super::query::{ConsistencyScoreResult, GetConsistencyScoreQuery};

/// Handler for GetConsistencyScoreQuery.
///
/// Computes a consistency score over a date range by:
/// 1. Determining the date range: [today - (days-1), today] (inclusive)
/// 2. Bulk-fetching metadata, logs, and goals via DailyAchievementEvaluator
/// 3. Counting days with goals and days achieved
/// 4. Returning the raw counts (percentage calculation done at API layer)
pub struct GetConsistencyScoreQueryHandler<R, C, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    C: Clock + Send + Sync + 'static,
    E: DailyAchievementEvaluator + Send + Sync + 'static,
{
    repository: Arc<R>,
    clock: Arc<C>,
    evaluator: Arc<E>,
}

impl<R, C, E> GetConsistencyScoreQueryHandler<R, C, E>
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

impl<R, C, E> QueryHandler<GetConsistencyScoreQuery> for GetConsistencyScoreQueryHandler<R, C, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    C: Clock + Send + Sync + 'static,
    E: DailyAchievementEvaluator + Send + Sync + 'static,
{
    fn handle(&self, query: GetConsistencyScoreQuery) -> ApplicationResult<ConsistencyScoreResult> {
        // Determine the date range: [today - (days-1), today]
        let today = self.clock.today();
        let days_to_subtract = query.days.saturating_sub(1);
        let start = today
            .checked_sub_days(Days::new(days_to_subtract as u64))
            .unwrap_or(today);
        let range = DateRange::new(start, today)?;

        // Fetch statistic metadata
        let stat = self.repository.find_metadata_by_id(query.statistic_id)?;

        // Evaluate each day in the range using the shared evaluator
        let daily_evals = self.evaluator.eval_range(&stat, range)?;

        // Count days with goals and days achieved
        let mut days_with_goal = 0u32;
        let mut days_achieved = 0u32;

        for day_eval in daily_evals.iter() {
            if day_eval.has_goal {
                days_with_goal += 1;
                if day_eval.achieved {
                    days_achieved += 1;
                }
            }
        }

        Ok(ConsistencyScoreResult {
            statistic_id: query.statistic_id,
            days_with_goal,
            days_achieved,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_core::shared::date_range::DateRange;
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
    fn test_consistency_score_all_achieved() {
        // Arrange
        let today = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
        let stat_id = StatisticId(1);

        let mut mock_repo = MockStatisticRepository::new();
        let stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(stat_id, "Test".into(), "test_stat".into()),
            None,
            today,
        ));

        mock_repo
            .expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        let mut mock_evaluator = MockDailyAchievementEvaluator::new();
        mock_evaluator
            .expect_eval_range()
            .times(1)
            .returning(|_, _| {
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
        let handler = GetConsistencyScoreQueryHandler::new(
            Arc::new(mock_repo),
            clock,
            Arc::new(mock_evaluator),
        );

        // Act
        let query = GetConsistencyScoreQuery {
            statistic_id: stat_id,
            days: 3,
        };
        let result = handler.handle(query);

        // Assert
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.days_with_goal, 3);
        assert_eq!(result.days_achieved, 3);
    }

    #[test]
    fn test_consistency_score_partial_achievement() {
        // Arrange
        let today = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
        let stat_id = StatisticId(1);

        let mut mock_repo = MockStatisticRepository::new();
        let stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(stat_id, "Test".into(), "test_stat".into()),
            None,
            today,
        ));

        mock_repo
            .expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        let mut mock_evaluator = MockDailyAchievementEvaluator::new();
        mock_evaluator
            .expect_eval_range()
            .times(1)
            .returning(|_, _| {
                Ok(vec![
                    DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, 8).unwrap(),
                        has_goal: true,
                        achieved: true,
                    },
                    DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, 9).unwrap(),
                        has_goal: true,
                        achieved: false,
                    },
                    DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
                        has_goal: true,
                        achieved: true,
                    },
                ])
            });

        let clock = Arc::new(StubClock { fixed_date: today });
        let handler = GetConsistencyScoreQueryHandler::new(
            Arc::new(mock_repo),
            clock,
            Arc::new(mock_evaluator),
        );

        // Act
        let query = GetConsistencyScoreQuery {
            statistic_id: stat_id,
            days: 3,
        };
        let result = handler.handle(query);

        // Assert
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.days_with_goal, 3);
        assert_eq!(result.days_achieved, 2); // 2 out of 3 days achieved
    }

    #[test]
    fn test_consistency_score_excludes_no_goal_days() {
        // Arrange
        let today = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
        let stat_id = StatisticId(1);

        let mut mock_repo = MockStatisticRepository::new();
        let stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(stat_id, "Test".into(), "test_stat".into()),
            None,
            today,
        ));

        mock_repo
            .expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        let mut mock_evaluator = MockDailyAchievementEvaluator::new();
        mock_evaluator
            .expect_eval_range()
            .times(1)
            .returning(|_, _| {
                Ok(vec![
                    DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, 8).unwrap(),
                        has_goal: false,
                        achieved: false,
                    },
                    DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, 9).unwrap(),
                        has_goal: true,
                        achieved: true,
                    },
                    DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
                        has_goal: true,
                        achieved: false,
                    },
                ])
            });

        let clock = Arc::new(StubClock { fixed_date: today });
        let handler = GetConsistencyScoreQueryHandler::new(
            Arc::new(mock_repo),
            clock,
            Arc::new(mock_evaluator),
        );

        // Act
        let query = GetConsistencyScoreQuery {
            statistic_id: stat_id,
            days: 3,
        };
        let result = handler.handle(query);

        // Assert
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.days_with_goal, 2); // Only 2 days had goals
        assert_eq!(result.days_achieved, 1); // Only 1 of those was achieved
    }
}
