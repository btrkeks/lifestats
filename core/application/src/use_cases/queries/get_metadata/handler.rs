use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

use super::query::GetMetadataQuery;

/// Handler for GetMetadataQuery.
pub struct GetMetadataQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    repository: Arc<R>,
}

impl<R> GetMetadataQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R> QueryHandler<GetMetadataQuery> for GetMetadataQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    fn handle(&self, query: GetMetadataQuery) -> ApplicationResult<StatisticMetadata> {
        let statistic = self.repository.find_metadata_by_id(query.statistic_id)?;
        Ok(statistic.get_metadata().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::any_statistic::AnyStatistic;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

    #[test]
    fn test_get_metadata_returns_metadata() {
        let mut repo = MockStatisticRepository::new();
        let metadata = StatisticMetadata::new(StatisticId(1), "cat".into(), "name".into());
        let statistic = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            metadata.clone(),
            None,
            Local::now().date_naive(),
        ));

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(statistic));

        let handler = GetMetadataQueryHandler::new(Arc::new(repo));
        let result = handler
            .handle(GetMetadataQuery {
                statistic_id: StatisticId(1),
            })
            .unwrap();
        assert_eq!(result.name(), "name");
    }
}
