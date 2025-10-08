use std::sync::Arc;

use crate::{cqrs::QueryHandler, error::ApplicationResult};
use lifestats_core::statistic::{
    any_statistic::AnyStatistic, ports::statistic_repository::StatisticRepository,
    value_objects::statistic_metadata::StatisticMetadata,
};

use super::query::GetTimeSpansMetadataQuery;

/// Handler that returns metadata for all TimeSpans statistics.
///
/// Logger applications use this to present the user with a list of
/// statistics that support heartbeat sessions (time tracking).
pub struct GetTimeSpansMetadataQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    repository: Arc<R>,
}

impl<R> GetTimeSpansMetadataQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R> QueryHandler<GetTimeSpansMetadataQuery> for GetTimeSpansMetadataQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    fn handle(
        &self,
        _query: GetTimeSpansMetadataQuery,
    ) -> ApplicationResult<Vec<StatisticMetadata>> {
        let statistics = self.repository.find_all_metadata()?;

        let metadata = statistics
            .into_iter()
            .filter_map(|stat| match &stat {
                AnyStatistic::TimeSpans(_) => Some(stat.get_metadata().clone()),
                _ => None,
            })
            .collect();

        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use lifestats_core::{
        shared::ids::StatisticId,
        statistic::{
            ports::statistic_repository::MockStatisticRepository,
            types::logable_statistics::{
                integer_amount::IntegerAmountStatistic, time_spans::TimeSpansStatistic,
            },
            value_objects::statistic_metadata::StatisticMetadata,
        },
    };

    #[test]
    fn returns_only_time_spans_metadata() {
        let mut repo = MockStatisticRepository::new();
        let today = Local::now().date_naive();

        let time_spans_meta =
            StatisticMetadata::new(StatisticId(1), "time".into(), "Practice Piano".into());
        let integer_meta =
            StatisticMetadata::new(StatisticId(2), "amount".into(), "Push-ups".into());

        let time_spans_stat = AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            time_spans_meta.clone(),
            None,
            today,
        ));
        let integer_stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            integer_meta.clone(),
            None,
            today,
        ));

        repo.expect_find_all_metadata()
            .times(1)
            .returning(move || Ok(vec![time_spans_stat.clone(), integer_stat.clone()]));

        let handler = GetTimeSpansMetadataQueryHandler::new(Arc::new(repo));
        let result = handler.handle(GetTimeSpansMetadataQuery);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.len(), 1);
        let first = &metadata[0];
        assert_eq!(first.id(), time_spans_meta.id());
        assert_eq!(first.name(), time_spans_meta.name());
        assert_eq!(first.category(), time_spans_meta.category());
    }

    #[test]
    fn returns_empty_when_no_time_spans_statistics_exist() {
        let mut repo = MockStatisticRepository::new();

        repo.expect_find_all_metadata()
            .times(1)
            .returning(|| Ok(vec![]));

        let handler = GetTimeSpansMetadataQueryHandler::new(Arc::new(repo));
        let result = handler.handle(GetTimeSpansMetadataQuery);

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert!(metadata.is_empty());
    }
}
