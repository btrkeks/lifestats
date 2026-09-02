use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::dto::StatisticSummary;
use crate::error::ApplicationResult;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

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
    fn handle(&self, _query: GetAllMetadataQuery) -> ApplicationResult<Vec<StatisticSummary>> {
        let statistics = self.repository.find_all_metadata()?;
        Ok(statistics.iter().map(StatisticSummary::from).collect())
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
    fn test_get_all_metadata_returns_summary_with_type() {
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
        assert_eq!(result[0].metadata.name(), "name");
        assert_eq!(
            result[0].statistic_type,
            crate::dto::StatisticType::IntegerAmount
        );
        assert_eq!(result[0].default_goal, None);
    }

    #[test]
    fn test_get_all_metadata_includes_default_goal() {
        use lifestats_core::statistic::value_objects::goals::amount_goal::AmountGoal;
        use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

        let mut repo = MockStatisticRepository::new();
        let today = Local::now().date_naive();
        let metadata = vec![
            AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
                StatisticMetadata::new(StatisticId(1), "cat".into(), "with goal".into()),
                Some(AmountGoal::new(10, true)),
                today,
            )),
            AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
                StatisticMetadata::new(StatisticId(2), "cat".into(), "without goal".into()),
                None,
                today,
            )),
        ];

        repo.expect_find_all_metadata()
            .times(1)
            .return_once(move || Ok(metadata.clone()));

        let handler = GetAllMetadataQueryHandler::new(Arc::new(repo));
        let result = handler.handle(GetAllMetadataQuery).unwrap();

        assert_eq!(
            result[0].default_goal,
            Some(serde_json::json!({"target_amount": 10, "is_at_least": true}))
        );
        assert_eq!(result[1].default_goal, None);
    }
}
