use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use super::query::GetStatisticByIdQuery;

/// Handler for GetStatisticByIdQuery
pub struct GetStatisticByIdQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    repository: Arc<R>,
}

impl<R> GetStatisticByIdQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R> QueryHandler<GetStatisticByIdQuery> for GetStatisticByIdQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    fn handle(&self, query: GetStatisticByIdQuery) -> ApplicationResult<AnyStatistic> {
        Ok(self.repository.find_metadata_by_id(query.statistic_id)?)
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
    fn test_get_statistic_by_id_found() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let stat_id = StatisticId(42);

        let stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(stat_id, "category".into(), "test_stat".into()),
            None,
            Local::now().date_naive(),
        ));

        let expected = stat.clone();

        repo.expect_find_metadata_by_id()
            .withf(move |id| *id == stat_id)
            .times(1)
            .returning(move |_| Ok(expected.clone()));

        let handler = GetStatisticByIdQueryHandler::new(Arc::new(repo));
        let query = GetStatisticByIdQuery {
            statistic_id: stat_id,
        };

        // Act
        let result = handler.handle(query);

        // Assert
        assert!(result.is_ok());
        let retrieved = result.unwrap();
        assert_eq!(retrieved.get_metadata().id(), stat_id);
    }
}
