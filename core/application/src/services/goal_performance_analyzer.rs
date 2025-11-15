use std::sync::Arc;

use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::shared::percentage::Percentage;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use crate::dto::goal_performance::{GoalPerformanceDto, TrendDto};
use crate::error::{ApplicationError, ApplicationResult};
use crate::services::daily_achievement_evaluator::DailyAchievementEvaluator;
use crate::services::ewma_calculator::Ewma;

/// Configuration constants for EWMA trend calculation
const FAST_EWMA_SPAN: u32 = 3;
const SLOW_EWMA_SPAN: u32 = 7;
const MIN_DATA_POINTS_FOR_TREND: usize = 14;
const TREND_EPSILON: f64 = 1.0; // Percentage points

/// Analyzes goal performance over a date range using EWMA-based trend detection
///
/// This service builds on top of `DailyAchievementEvaluator` to provide:
/// - Overall achievement score (average percentage across the date range)
/// - Trend indicator (improving, declining, stable, or insufficient data)
///
/// The trend is calculated using fast and slow exponentially weighted moving averages:
/// - Fast EWMA (3-day span) responds quickly to recent changes
/// - Slow EWMA (7-day span) provides a stable baseline
/// - Comparing the two reveals whether performance is improving or declining
pub struct GoalPerformanceAnalyzer<E, R>
where
    E: DailyAchievementEvaluator + ?Sized,
    R: StatisticRepository + ?Sized,
{
    evaluator: Arc<E>,
    repo: Arc<R>,
}

impl<E, R> GoalPerformanceAnalyzer<E, R>
where
    E: DailyAchievementEvaluator,
    R: StatisticRepository + Send + Sync + 'static,
{
    /// Create a new goal performance analyzer
    ///
    /// # Arguments
    /// * `evaluator` - Service for evaluating daily goal achievements
    /// * `repo` - Repository for loading statistic metadata
    pub fn new(evaluator: Arc<E>, repo: Arc<R>) -> Self {
        Self { evaluator, repo }
    }

    /// Analyze goal performance over the specified date range
    ///
    /// # Arguments
    /// * `statistic_id` - The statistic to analyze
    /// * `date_range` - Date range for the analysis
    ///
    /// # Returns
    /// - `GoalPerformanceDto` containing overall score and trend
    /// - `InsufficientData` error if the date range has no data
    ///
    /// # Semantics for days without goals
    /// - Days with `percentage_achieved: None` (suppressed goals, no goal, etc.) are treated
    ///   as 100% achievement for the purposes of performance analysis
    pub fn analyze(
        &self,
        statistic_id: StatisticId,
        date_range: DateRange,
    ) -> ApplicationResult<GoalPerformanceDto> {
        // Load statistic metadata
        let stat = self.repo.find_metadata_by_id(statistic_id)?;

        // Get daily evaluations from the canonical evaluator
        let day_evals = self.evaluator.eval_range(&stat, date_range)?;

        // Map DayEval -> daily percentage scores
        // Treat None (suppressed/no-goal) as 100% for this analysis
        let daily_scores: Vec<f64> = day_evals
            .iter()
            .map(|d| {
                d.percentage_achieved
                    .unwrap_or_else(|| Percentage::new(100))
                    .as_fraction()
                    * 100.0
            })
            .collect();

        // Check for empty data
        if daily_scores.is_empty() {
            return Err(ApplicationError::InsufficientData(
                "No data available in the selected date range".to_string(),
            ));
        }

        // Calculate overall score (average)
        let overall_score_value = daily_scores.iter().sum::<f64>() / daily_scores.len() as f64;
        let overall_score = Percentage::new(overall_score_value.round() as u8);

        // Calculate trend if we have enough data points
        let trend = if daily_scores.len() < MIN_DATA_POINTS_FOR_TREND {
            TrendDto::InsufficientData
        } else {
            self.calculate_trend(&daily_scores)
        };

        Ok(GoalPerformanceDto {
            overall_score,
            trend,
        })
    }

    /// Calculate trend using EWMA comparison
    fn calculate_trend(&self, daily_scores: &[f64]) -> TrendDto {
        // Initialize both EWMAs with the first value
        let initial = daily_scores[0];
        let mut fast_ewma = Ewma::from_span(FAST_EWMA_SPAN, initial);
        let mut slow_ewma = Ewma::from_span(SLOW_EWMA_SPAN, initial);

        // Process all subsequent values
        for &score in &daily_scores[1..] {
            fast_ewma.add(score);
            slow_ewma.add(score);
        }

        let fast_val = fast_ewma.value();
        let slow_val = slow_ewma.value();
        let diff = fast_val - slow_val;

        // Check if the difference is within the stability threshold
        if diff.abs() < TREND_EPSILON {
            TrendDto::Stable
        } else {
            // Calculate normalized strength (0.0 to 1.0)
            let strength = (diff.abs() / 100.0).clamp(0.0, 1.0);

            if diff > 0.0 {
                TrendDto::Improving { strength }
            } else {
                TrendDto::Declining { strength }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_core::statistic::any_statistic::AnyStatistic;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::habit::HabitStatistic;

    use crate::services::daily_achievement_evaluator::{DayEval, MockDailyAchievementEvaluator};

    fn mock_habit_statistic(id: u32) -> AnyStatistic {
        use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

        let metadata = StatisticMetadata::new(
            lifestats_core::shared::ids::StatisticId(id),
            "Test".to_string(),
            "Test Habit".to_string(),
        );

        AnyStatistic::Habit(HabitStatistic::new(
            metadata,
            None,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        ))
    }

    #[test]
    fn test_empty_data_returns_insufficient_data_error() {
        let stat_id = StatisticId(1);
        let date_range = DateRange::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
        )
        .unwrap();

        let mut mock_repo = MockStatisticRepository::new();
        let stat = mock_habit_statistic(1);
        mock_repo
            .expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        let mut mock_evaluator = MockDailyAchievementEvaluator::new();
        mock_evaluator
            .expect_eval_range()
            .times(1)
            .returning(|_, _| Ok(vec![])); // Empty result

        let analyzer = GoalPerformanceAnalyzer::new(Arc::new(mock_evaluator), Arc::new(mock_repo));

        let result = analyzer.analyze(stat_id, date_range);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ApplicationError::InsufficientData(_)
        ));
    }

    #[test]
    fn test_insufficient_data_for_trend() {
        let stat_id = StatisticId(1);
        let date_range = DateRange::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
        )
        .unwrap();

        let mut mock_repo = MockStatisticRepository::new();
        let stat = mock_habit_statistic(1);
        mock_repo
            .expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        // Only 5 data points (less than MIN_DATA_POINTS_FOR_TREND = 14)
        let mut mock_evaluator = MockDailyAchievementEvaluator::new();
        mock_evaluator
            .expect_eval_range()
            .times(1)
            .returning(|_, _| {
                Ok(vec![
                    DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                        has_goal: true,
                        achieved: true,
                        percentage_achieved: Some(Percentage::new(100)),
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
                        achieved: false,
                        percentage_achieved: Some(Percentage::new(0)),
                    },
                    DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, 4).unwrap(),
                        has_goal: true,
                        achieved: true,
                        percentage_achieved: Some(Percentage::new(100)),
                    },
                    DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, 5).unwrap(),
                        has_goal: true,
                        achieved: true,
                        percentage_achieved: Some(Percentage::new(100)),
                    },
                ])
            });

        let analyzer = GoalPerformanceAnalyzer::new(Arc::new(mock_evaluator), Arc::new(mock_repo));

        let result = analyzer.analyze(stat_id, date_range).unwrap();

        assert_eq!(result.overall_score, Percentage::new(80)); // 400/5 = 80%
        assert_eq!(result.trend, TrendDto::InsufficientData);
    }

    #[test]
    fn test_improving_trend() {
        let stat_id = StatisticId(1);
        let date_range = DateRange::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 20).unwrap(),
        )
        .unwrap();

        let mut mock_repo = MockStatisticRepository::new();
        let stat = mock_habit_statistic(1);
        mock_repo
            .expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        // Simulate improving performance: start low, end high
        let mut mock_evaluator = MockDailyAchievementEvaluator::new();
        mock_evaluator
            .expect_eval_range()
            .times(1)
            .returning(|_, _| {
                let mut evals = Vec::new();
                for day in 1..=20 {
                    let percentage = if day <= 10 {
                        50 // First 10 days: 50%
                    } else {
                        90 // Last 10 days: 90%
                    };
                    evals.push(DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, day).unwrap(),
                        has_goal: true,
                        achieved: percentage == 100,
                        percentage_achieved: Some(Percentage::new(percentage)),
                    });
                }
                Ok(evals)
            });

        let analyzer = GoalPerformanceAnalyzer::new(Arc::new(mock_evaluator), Arc::new(mock_repo));

        let result = analyzer.analyze(stat_id, date_range).unwrap();

        assert_eq!(result.overall_score, Percentage::new(70)); // (500 + 900) / 20 = 70%
        match result.trend {
            TrendDto::Improving { strength } => {
                assert!(strength > 0.0);
            }
            _ => panic!("Expected improving trend"),
        }
    }

    #[test]
    fn test_declining_trend() {
        let stat_id = StatisticId(1);
        let date_range = DateRange::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 20).unwrap(),
        )
        .unwrap();

        let mut mock_repo = MockStatisticRepository::new();
        let stat = mock_habit_statistic(1);
        mock_repo
            .expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        // Simulate declining performance: start high, end low
        let mut mock_evaluator = MockDailyAchievementEvaluator::new();
        mock_evaluator
            .expect_eval_range()
            .times(1)
            .returning(|_, _| {
                let mut evals = Vec::new();
                for day in 1..=20 {
                    let percentage = if day <= 10 {
                        90 // First 10 days: 90%
                    } else {
                        50 // Last 10 days: 50%
                    };
                    evals.push(DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, day).unwrap(),
                        has_goal: true,
                        achieved: percentage == 100,
                        percentage_achieved: Some(Percentage::new(percentage)),
                    });
                }
                Ok(evals)
            });

        let analyzer = GoalPerformanceAnalyzer::new(Arc::new(mock_evaluator), Arc::new(mock_repo));

        let result = analyzer.analyze(stat_id, date_range).unwrap();

        assert_eq!(result.overall_score, Percentage::new(70)); // (900 + 500) / 20 = 70%
        match result.trend {
            TrendDto::Declining { strength } => {
                assert!(strength > 0.0);
            }
            _ => panic!("Expected declining trend"),
        }
    }

    #[test]
    fn test_stable_trend() {
        let stat_id = StatisticId(1);
        let date_range = DateRange::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 20).unwrap(),
        )
        .unwrap();

        let mut mock_repo = MockStatisticRepository::new();
        let stat = mock_habit_statistic(1);
        mock_repo
            .expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        // Stable performance: consistently 80%
        let mut mock_evaluator = MockDailyAchievementEvaluator::new();
        mock_evaluator
            .expect_eval_range()
            .times(1)
            .returning(|_, _| {
                let mut evals = Vec::new();
                for day in 1..=20 {
                    evals.push(DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, day).unwrap(),
                        has_goal: true,
                        achieved: false,
                        percentage_achieved: Some(Percentage::new(80)),
                    });
                }
                Ok(evals)
            });

        let analyzer = GoalPerformanceAnalyzer::new(Arc::new(mock_evaluator), Arc::new(mock_repo));

        let result = analyzer.analyze(stat_id, date_range).unwrap();

        assert_eq!(result.overall_score, Percentage::new(80));
        assert_eq!(result.trend, TrendDto::Stable);
    }

    #[test]
    fn test_suppressed_goals_treated_as_100_percent() {
        let stat_id = StatisticId(1);
        let date_range = DateRange::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 20).unwrap(),
        )
        .unwrap();

        let mut mock_repo = MockStatisticRepository::new();
        let stat = mock_habit_statistic(1);
        mock_repo
            .expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        // Mix of actual percentages and suppressed goals (None)
        let mut mock_evaluator = MockDailyAchievementEvaluator::new();
        mock_evaluator
            .expect_eval_range()
            .times(1)
            .returning(|_, _| {
                let mut evals = Vec::new();
                for day in 1..=20 {
                    let percentage_achieved = if day % 5 == 0 {
                        None // Suppressed goal every 5th day
                    } else {
                        Some(Percentage::new(60))
                    };
                    evals.push(DayEval {
                        date: NaiveDate::from_ymd_opt(2025, 1, day).unwrap(),
                        has_goal: percentage_achieved.is_some(),
                        achieved: false,
                        percentage_achieved,
                    });
                }
                Ok(evals)
            });

        let analyzer = GoalPerformanceAnalyzer::new(Arc::new(mock_evaluator), Arc::new(mock_repo));

        let result = analyzer.analyze(stat_id, date_range).unwrap();

        // 16 days at 60% + 4 days at 100% (suppressed) = (960 + 400) / 20 = 68%
        assert_eq!(result.overall_score, Percentage::new(68));
    }
}
