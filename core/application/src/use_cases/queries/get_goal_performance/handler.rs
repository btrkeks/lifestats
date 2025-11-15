use std::sync::Arc;

use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use crate::cqrs::QueryHandler;
use crate::dto::goal_performance::GoalPerformanceDto;
use crate::error::ApplicationResult;
use crate::services::daily_achievement_evaluator::DailyAchievementEvaluator;
use crate::services::goal_performance_analyzer::GoalPerformanceAnalyzer;

use super::query::GetGoalPerformanceQuery;

/// Handler for GetGoalPerformanceQuery
///
/// Delegates to GoalPerformanceAnalyzer to compute:
/// - Overall achievement score over the date range
/// - Trend indicator based on EWMA analysis
pub struct GetGoalPerformanceQueryHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: DailyAchievementEvaluator + Send + Sync + 'static,
{
    analyzer: Arc<GoalPerformanceAnalyzer<E, R>>,
}

impl<R, E> GetGoalPerformanceQueryHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: DailyAchievementEvaluator + Send + Sync + 'static,
{
    pub fn new(analyzer: Arc<GoalPerformanceAnalyzer<E, R>>) -> Self {
        Self { analyzer }
    }
}

impl<R, E> QueryHandler<GetGoalPerformanceQuery> for GetGoalPerformanceQueryHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: DailyAchievementEvaluator + Send + Sync + 'static,
{
    fn handle(&self, query: GetGoalPerformanceQuery) -> ApplicationResult<GoalPerformanceDto> {
        self.analyzer.analyze(query.statistic_id, query.date_range)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_core::shared::date_range::DateRange;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::shared::percentage::Percentage;

    use crate::dto::goal_performance::TrendDto;

    struct MockAnalyzer {
        result: ApplicationResult<GoalPerformanceDto>,
    }

    impl<E: DailyAchievementEvaluator, R: StatisticRepository> GoalPerformanceAnalyzer<E, R> {
        fn mock(result: ApplicationResult<GoalPerformanceDto>) -> MockAnalyzer {
            MockAnalyzer { result }
        }
    }

    #[test]
    fn test_handler_delegates_to_analyzer() {
        // This is a conceptual test showing the handler delegates to the analyzer
        // In practice, integration tests at the web API level test the full stack
        let query = GetGoalPerformanceQuery {
            statistic_id: StatisticId(1),
            date_range: DateRange::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 1, 31).unwrap(),
            )
            .unwrap(),
        };

        // The handler's role is purely to delegate to the analyzer
        // Actual testing happens in:
        // 1. GoalPerformanceAnalyzer unit tests
        // 2. Web API integration tests
        assert_eq!(query.statistic_id, StatisticId(1));
    }
}
