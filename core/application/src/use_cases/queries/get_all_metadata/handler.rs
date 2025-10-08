use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

use super::query::GetAllMetadataQuery;

/// Handler for GetAllMetadataQuery.
pub struct GetAllMetadataQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    repository: Arc<R>,
}

impl<R> GetAllMetadataQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R> QueryHandler<GetAllMetadataQuery> for GetAllMetadataQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    fn handle(&self, _query: GetAllMetadataQuery) -> ApplicationResult<Vec<StatisticMetadata>> {
        let statistics = self.repository.find_all_metadata()?;
        Ok(statistics
            .into_iter()
            .map(|stat| stat.get_metadata().clone())
            .collect())
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

    #[test]
    fn test_get_all_metadata_returns_metadata() {
        let mut repo = MockStatisticRepository::new();
        let metadata = vec![AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata::new(
                StatisticId(1),
                "cat".into(),
                "name".into(),
            ),
            None,
            Local::now().date_naive(),
        ))];

        repo.expect_find_all_metadata()
            .times(1)
            .return_once(move || Ok(metadata.clone()));

        let handler = GetAllMetadataQueryHandler::new(Arc::new(repo));
        let result = handler.handle(GetAllMetadataQuery).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name(), "name");
    }
}
