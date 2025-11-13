use std::sync::Arc;

use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use crate::services::{compute_mean_percentage, DailyAchievementEvaluator};

use super::query::{GetStatisticScoreQuery, StatisticScoreResult};

/// Handler for GetStatisticScoreQuery.
///
/// Responsibilities:
/// - Delegate per-day evaluation to DailyAchievementEvaluator
/// - Aggregate daily percentages via compute_mean_percentage
/// - Return a simple, application-level result struct
pub struct GetStatisticScoreQueryHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: DailyAchievementEvaluator + Send + Sync + 'static,
{
    repository: Arc<R>,
    evaluator: Arc<E>,
}

impl<R, E> GetStatisticScoreQueryHandler<R, E>
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

impl<R, E> QueryHandler<GetStatisticScoreQuery> for GetStatisticScoreQueryHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: DailyAchievementEvaluator + Send + Sync + 'static,
{
    fn handle(&self, query: GetStatisticScoreQuery) -> ApplicationResult<StatisticScoreResult> {
        // Ensure statistic exists; lets us fail fast for invalid IDs
        let statistic = self.repository.find_metadata_by_id(query.statistic_id)?;

        // Bulk-evaluate goal achievement for the full date range
        let evals = self.evaluator.eval_range(&statistic, query.date_range)?;

        // Aggregate percentages
        let mean_percentage = compute_mean_percentage(&evals);

        let days_with_goal = evals
            .iter()
            .filter(|e| e.percentage_achieved.is_some())
            .count() as u32;

        Ok(StatisticScoreResult {
            statistic_id: query.statistic_id,
            start_date: query.date_range.start(),
            end_date: query.date_range.end(),
            mean_percentage_achieved: mean_percentage,
            days_with_goal,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_core::shared::date_range::DateRange;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::shared::percentage::Percentage;
    use lifestats_core::statistic::any_statistic::AnyStatistic;
    use lifestats_core::statistic::ports::error::RepositoryError;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

    use crate::services::{DayEval, MockDailyAchievementEvaluator};

    #[test]
    fn test_get_statistic_score_no_goals_in_range() {
        // Arrange
        let stat_id = StatisticId(1);
        let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2025, 1, 3).unwrap();
        let date_range = DateRange::new(start_date, end_date).unwrap();

        let mut mock_repo = MockStatisticRepository::new();
        let stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(stat_id, "Test".into(), "test_stat".into()),
            None,
            start_date,
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
                        date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                        has_goal: false,
                        achieved: false,
                        percentage_achieved: None,
                    },
                    DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                        has_goal: false,
                        achieved: false,
                        percentage_achieved: None,
                    },
                    DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
                        has_goal: false,
                        achieved: false,
                        percentage_achieved: None,
                    },
                ])
            });

        let handler =
            GetStatisticScoreQueryHandler::new(Arc::new(mock_repo), Arc::new(mock_evaluator));

        // Act
        let query = GetStatisticScoreQuery {
            statistic_id: stat_id,
            date_range,
        };
        let result = handler.handle(query);

        // Assert
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.statistic_id, stat_id);
        assert_eq!(result.start_date, start_date);
        assert_eq!(result.end_date, end_date);
        assert_eq!(result.mean_percentage_achieved, None);
        assert_eq!(result.days_with_goal, 0);
    }

    #[test]
    fn test_get_statistic_score_mixed_percentages() {
        // Arrange
        let stat_id = StatisticId(1);
        let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2025, 1, 3).unwrap();
        let date_range = DateRange::new(start_date, end_date).unwrap();

        let mut mock_repo = MockStatisticRepository::new();
        let stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(stat_id, "Test".into(), "test_stat".into()),
            None,
            start_date,
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
                        date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                        has_goal: true,
                        achieved: false,
                        percentage_achieved: Some(Percentage::new(50)),
                    },
                    DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                        has_goal: true,
                        achieved: true,
                        percentage_achieved: Some(Percentage::new(100)),
                    },
                    DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
                        has_goal: true,
                        achieved: true,
                        percentage_achieved: Some(Percentage::new(150)),
                    },
                ])
            });

        let handler =
            GetStatisticScoreQueryHandler::new(Arc::new(mock_repo), Arc::new(mock_evaluator));

        // Act
        let query = GetStatisticScoreQuery {
            statistic_id: stat_id,
            date_range,
        };
        let result = handler.handle(query);

        // Assert
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.statistic_id, stat_id);
        assert_eq!(result.start_date, start_date);
        assert_eq!(result.end_date, end_date);
        // (50 + 100 + 150) / 3 = 300 / 3 = 100
        assert_eq!(result.mean_percentage_achieved, Some(Percentage::new(100)));
        assert_eq!(result.days_with_goal, 3);
    }

    #[test]
    fn test_get_statistic_score_invalid_statistic_id() {
        // Arrange
        let stat_id = StatisticId(999);
        let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2025, 1, 3).unwrap();
        let date_range = DateRange::new(start_date, end_date).unwrap();

        let mut mock_repo = MockStatisticRepository::new();
        mock_repo
            .expect_find_metadata_by_id()
            .times(1)
            .returning(|_| Err(RepositoryError::NotFound));

        let mock_evaluator = MockDailyAchievementEvaluator::new();

        let handler =
            GetStatisticScoreQueryHandler::new(Arc::new(mock_repo), Arc::new(mock_evaluator));

        // Act
        let query = GetStatisticScoreQuery {
            statistic_id: stat_id,
            date_range,
        };
        let result = handler.handle(query);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_get_statistic_score_excludes_days_without_goals() {
        // Arrange
        let stat_id = StatisticId(1);
        let start_date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(2025, 1, 4).unwrap();
        let date_range = DateRange::new(start_date, end_date).unwrap();

        let mut mock_repo = MockStatisticRepository::new();
        let stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(stat_id, "Test".into(), "test_stat".into()),
            None,
            start_date,
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
                        date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                        has_goal: false,
                        achieved: false,
                        percentage_achieved: None,
                    },
                    DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                        has_goal: true,
                        achieved: true,
                        percentage_achieved: Some(Percentage::new(80)),
                    },
                    DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
                        has_goal: true,
                        achieved: true,
                        percentage_achieved: Some(Percentage::new(100)),
                    },
                    DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, 4).unwrap(),
                        has_goal: false,
                        achieved: false,
                        percentage_achieved: None,
                    },
                ])
            });

        let handler =
            GetStatisticScoreQueryHandler::new(Arc::new(mock_repo), Arc::new(mock_evaluator));

        // Act
        let query = GetStatisticScoreQuery {
            statistic_id: stat_id,
            date_range,
        };
        let result = handler.handle(query);

        // Assert
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.statistic_id, stat_id);
        // Only days with goals count: (80 + 100) / 2 = 90
        assert_eq!(result.mean_percentage_achieved, Some(Percentage::new(90)));
        assert_eq!(result.days_with_goal, 2);
    }
}
