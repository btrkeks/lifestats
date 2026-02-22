use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use anyhow::anyhow;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use super::query::{GetIntegerAmountTotalQuery, IntegerAmountTotalResult};

pub struct GetIntegerAmountTotalQueryHandler<S>
where
    S: StatisticRepository + Send + Sync + 'static,
{
    repository: Arc<S>,
}

impl<S> GetIntegerAmountTotalQueryHandler<S>
where
    S: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(repository: Arc<S>) -> Self {
        Self { repository }
    }
}

impl<S> QueryHandler<GetIntegerAmountTotalQuery> for GetIntegerAmountTotalQueryHandler<S>
where
    S: StatisticRepository + Send + Sync + 'static,
{
    fn handle(
        &self,
        query: GetIntegerAmountTotalQuery,
    ) -> ApplicationResult<IntegerAmountTotalResult> {
        let statistic = self.repository.find_metadata_by_id(query.statistic_id)?;

        if !matches!(statistic, AnyStatistic::IntegerAmount(_)) {
            return Err(anyhow!(
                "Statistic {} is not an IntegerAmount type",
                query.statistic_id
            )
            .into());
        }

        let log_values = self
            .repository
            .find_log_values_in_range(query.statistic_id, query.date_range)?;

        let mut total = 0_i64;
        for dated_value in log_values {
            let amount = serde_json::from_value::<i64>(dated_value.value)
                .map_err(|e| anyhow!("Failed to parse IntegerAmount value: {}", e))?;
            total += amount;
        }

        Ok(IntegerAmountTotalResult { total })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_core::shared::date_range::DateRange;
    use lifestats_core::shared::dated_value::DatedValue;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::{
        habit::HabitStatistic, integer_amount::IntegerAmountStatistic,
    };
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use serde_json::json;

    #[test]
    fn test_get_integer_amount_total_sums_multiple_days() {
        let mut repo = MockStatisticRepository::new();
        let stat_id = StatisticId::from(1);

        let statistic = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(stat_id, "Test".into(), "Anki Reps".into()),
            None,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        ));

        repo.expect_find_metadata_by_id()
            .returning(move |_| Ok(statistic.clone()));

        repo.expect_find_log_values_in_range().returning(|_, _| {
            Ok(vec![
                DatedValue::new(NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(), json!(100)),
                DatedValue::new(NaiveDate::from_ymd_opt(2025, 10, 2).unwrap(), json!(66)),
            ])
        });

        let handler = GetIntegerAmountTotalQueryHandler::new(Arc::new(repo));
        let query = GetIntegerAmountTotalQuery {
            statistic_id: stat_id,
            date_range: DateRange::new(
                NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 10, 10).unwrap(),
            )
            .unwrap(),
        };

        let result = handler.handle(query).unwrap();
        assert_eq!(result.total, 166);
    }

    #[test]
    fn test_rejects_non_integer_amount_statistic() {
        let mut repo = MockStatisticRepository::new();
        let stat_id = StatisticId::from(1);

        let statistic = AnyStatistic::Habit(HabitStatistic::new(
            StatisticMetadata::new(stat_id, "Test".into(), "Daily Habit".into()),
            None,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        ));

        repo.expect_find_metadata_by_id()
            .returning(move |_| Ok(statistic.clone()));

        let handler = GetIntegerAmountTotalQueryHandler::new(Arc::new(repo));
        let query = GetIntegerAmountTotalQuery {
            statistic_id: stat_id,
            date_range: DateRange::new(
                NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 10, 10).unwrap(),
            )
            .unwrap(),
        };

        let result = handler.handle(query);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not an IntegerAmount type"));
    }

    #[test]
    fn test_empty_date_range_returns_zero() {
        let mut repo = MockStatisticRepository::new();
        let stat_id = StatisticId::from(1);

        let statistic = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(stat_id, "Test".into(), "Count".into()),
            None,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        ));

        repo.expect_find_metadata_by_id()
            .returning(move |_| Ok(statistic.clone()));
        repo.expect_find_log_values_in_range()
            .returning(|_, _| Ok(vec![]));

        let handler = GetIntegerAmountTotalQueryHandler::new(Arc::new(repo));
        let query = GetIntegerAmountTotalQuery {
            statistic_id: stat_id,
            date_range: DateRange::new(
                NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 10, 10).unwrap(),
            )
            .unwrap(),
        };

        let result = handler.handle(query).unwrap();
        assert_eq!(result.total, 0);
    }

    #[test]
    fn test_invalid_persisted_value_returns_error() {
        let mut repo = MockStatisticRepository::new();
        let stat_id = StatisticId::from(1);

        let statistic = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(stat_id, "Test".into(), "Count".into()),
            None,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        ));

        repo.expect_find_metadata_by_id()
            .returning(move |_| Ok(statistic.clone()));
        repo.expect_find_log_values_in_range().returning(|_, _| {
            Ok(vec![DatedValue::new(
                NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(),
                json!({"oops":"not an int"}),
            )])
        });

        let handler = GetIntegerAmountTotalQueryHandler::new(Arc::new(repo));
        let query = GetIntegerAmountTotalQuery {
            statistic_id: stat_id,
            date_range: DateRange::new(
                NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 10, 10).unwrap(),
            )
            .unwrap(),
        };

        let result = handler.handle(query);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse IntegerAmount value"));
    }
}
