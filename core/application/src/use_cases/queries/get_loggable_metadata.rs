use std::sync::Arc;

use crate::cqrs::{Query, QueryHandler};
use crate::dto::StatisticSummary;
use crate::error::ApplicationResult;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

/// Query object to retrieve only statistics that can be logged directly by the user.
pub struct GetLoggableMetadataQuery;

impl Query for GetLoggableMetadataQuery {
    type Result = Vec<StatisticSummary>;
}

/// Handler responsible for filtering statistics to only loggable variants.
pub struct GetLoggableMetadataQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    statistic_repository: Arc<R>,
}

impl<R> GetLoggableMetadataQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(statistic_repository: Arc<R>) -> Self {
        Self {
            statistic_repository,
        }
    }
}

impl<R> QueryHandler<GetLoggableMetadataQuery> for GetLoggableMetadataQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    fn handle(&self, _query: GetLoggableMetadataQuery) -> ApplicationResult<Vec<StatisticSummary>> {
        let statistics = self.statistic_repository.find_all_metadata()?;

        let summaries = statistics
            .iter()
            .map(StatisticSummary::from)
            .filter(|summary| summary.statistic_type.is_manually_loggable())
            .collect();

        Ok(summaries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::StatisticType;
    use chrono::Local;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::any_statistic::AnyStatistic;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::derived_statistics::summing_statistic::SummingStatistic;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::types::logable_statistics::timed_amounts::TimedAmountsStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

    #[test]
    fn returns_only_loggable_summaries() {
        let mut repository = MockStatisticRepository::new();
        let today = Local::now().date_naive();

        let loggable_metadata =
            StatisticMetadata::new(StatisticId(1), "fitness".into(), "Push Ups".into());
        let derived_metadata =
            StatisticMetadata::new(StatisticId(2), "fitness".into(), "Total Time".into());

        let loggable_stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            loggable_metadata.clone(),
            None,
            today,
        ));
        let derived_stat = AnyStatistic::Summing(SummingStatistic::new(
            derived_metadata.clone(),
            Vec::new(),
            None,
            today,
        ));

        let source_owned_stat = AnyStatistic::TimedAmounts(TimedAmountsStatistic::new(
            StatisticMetadata::new(StatisticId(3), "OpenNutriTracker".into(), "Caffeine".into()),
            None,
            today,
        ));

        repository
            .expect_find_all_metadata()
            .times(1)
            .return_once(move || Ok(vec![loggable_stat, derived_stat, source_owned_stat]));

        let handler = GetLoggableMetadataQueryHandler::new(Arc::new(repository));
        let result = handler.handle(GetLoggableMetadataQuery).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].metadata.id(), loggable_metadata.id());
        assert_eq!(result[0].statistic_type, StatisticType::IntegerAmount);
    }
}
