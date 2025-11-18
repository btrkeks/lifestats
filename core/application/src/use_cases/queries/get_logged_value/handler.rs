use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::{ApplicationError, ApplicationResult};
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use super::query::GetLoggedValueQuery;

/// Handler for GetLoggedValueQuery.
pub struct GetLoggedValueQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    repository: Arc<R>,
}

impl<R> GetLoggedValueQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R> QueryHandler<GetLoggedValueQuery> for GetLoggedValueQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    fn handle(&self, query: GetLoggedValueQuery) -> ApplicationResult<Option<serde_json::Value>> {
        // 1. Verify statistic exists and get metadata
        let statistic = self.repository.find_metadata_by_id(query.statistic_id)?;

        // 2. Check if it's a derived statistic (which cannot be logged directly)
        match statistic {
            AnyStatistic::Summing(_) | AnyStatistic::TimeDifference(_) => {
                return Err(ApplicationError::InvalidInput(
                    "Cannot check logged value for derived statistics. \
                     Derived statistics (Summing, TimeDifference) are computed, not logged."
                        .to_string(),
                ));
            }
            _ => {} // Continue for logable statistics
        }

        // 3. Find the logged value for the specified date
        let log_value = self
            .repository
            .find_log_value(query.statistic_id, query.date)?;

        Ok(log_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::derived_statistics::summing_statistic::SummingStatistic;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use serde_json::json;

    fn mock_integer_statistic(id: u32, name: &str) -> AnyStatistic {
        let metadata =
            StatisticMetadata::new(StatisticId(id), "Test".to_string(), name.to_string());
        let today = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(metadata, None, today))
    }

    fn mock_summing_statistic(id: u32, name: &str) -> AnyStatistic {
        let metadata =
            StatisticMetadata::new(StatisticId(id), "Test".to_string(), name.to_string());
        let today = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        AnyStatistic::Summing(SummingStatistic::new(metadata, vec![], None, today))
    }

    #[test]
    fn test_get_logged_value_returns_some_when_value_exists() {
        let mut repo = MockStatisticRepository::new();
        let stat = mock_integer_statistic(1, "Push Ups");

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat.clone()));

        repo.expect_find_log_value()
            .times(1)
            .return_once(|_, _| Ok(Some(json!(42))));

        let handler = GetLoggedValueQueryHandler::new(Arc::new(repo));
        let result = handler
            .handle(GetLoggedValueQuery {
                statistic_id: StatisticId(1),
                date: NaiveDate::from_ymd_opt(2025, 11, 18).unwrap(),
            })
            .unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap(), json!(42));
    }

    #[test]
    fn test_get_logged_value_returns_none_when_no_value() {
        let mut repo = MockStatisticRepository::new();
        let stat = mock_integer_statistic(1, "Push Ups");

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat.clone()));

        repo.expect_find_log_value()
            .times(1)
            .return_once(|_, _| Ok(None));

        let handler = GetLoggedValueQueryHandler::new(Arc::new(repo));
        let result = handler
            .handle(GetLoggedValueQuery {
                statistic_id: StatisticId(1),
                date: NaiveDate::from_ymd_opt(2025, 11, 18).unwrap(),
            })
            .unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_get_logged_value_returns_error_for_derived_statistic() {
        let mut repo = MockStatisticRepository::new();
        let stat = mock_summing_statistic(1, "Total Work");

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat.clone()));

        let handler = GetLoggedValueQueryHandler::new(Arc::new(repo));
        let result = handler.handle(GetLoggedValueQuery {
            statistic_id: StatisticId(1),
            date: NaiveDate::from_ymd_opt(2025, 11, 18).unwrap(),
        });

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("derived statistics"));
    }

    #[test]
    fn test_get_logged_value_propagates_repository_error() {
        let mut repo = MockStatisticRepository::new();

        repo.expect_find_metadata_by_id().times(1).return_once(|_| {
            Err(lifestats_core::statistic::ports::error::RepositoryError::NotFound)
        });

        let handler = GetLoggedValueQueryHandler::new(Arc::new(repo));
        let result = handler.handle(GetLoggedValueQuery {
            statistic_id: StatisticId(999),
            date: NaiveDate::from_ymd_opt(2025, 11, 18).unwrap(),
        });

        assert!(result.is_err());
    }
}
