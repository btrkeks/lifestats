use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use super::query::GetAllStatisticsQuery;

/// Handler for GetAllStatisticsQuery
///
/// This handler simply delegates to the repository to fetch all statistic metadata.
/// No business logic or transformation is applied.
pub struct GetAllStatisticsQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    repository: Arc<R>,
}

impl<R> GetAllStatisticsQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R> QueryHandler<GetAllStatisticsQuery> for GetAllStatisticsQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    fn handle(&self, _query: GetAllStatisticsQuery) -> ApplicationResult<Vec<AnyStatistic>> {
        Ok(self.repository.find_all_metadata()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

    #[test]
    fn test_get_all_statistics_happy_path() {
        // Arrange
        let mut repo = MockStatisticRepository::new();

        let stat1 = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(StatisticId(1), "category".into(), "stat1".into()),
            None,
            Local::now().date_naive(),
        ));

        let stat2 = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(StatisticId(2), "category".into(), "stat2".into()),
            None,
            Local::now().date_naive(),
        ));

        let expected_stats = vec![stat1.clone(), stat2.clone()];

        repo.expect_find_all_metadata()
            .times(1)
            .returning(move || Ok(expected_stats.clone()));

        let handler = GetAllStatisticsQueryHandler::new(Arc::new(repo));
        let query = GetAllStatisticsQuery;

        // Act
        let result = handler.handle(query);

        // Assert
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.len(), 2);
    }

    #[test]
    fn test_get_all_statistics_empty() {
        // Arrange
        let mut repo = MockStatisticRepository::new();

        repo.expect_find_all_metadata()
            .times(1)
            .returning(|| Ok(vec![]));

        let handler = GetAllStatisticsQueryHandler::new(Arc::new(repo));
        let query = GetAllStatisticsQuery;

        // Act
        let result = handler.handle(query);

        // Assert
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.len(), 0);
    }
}
