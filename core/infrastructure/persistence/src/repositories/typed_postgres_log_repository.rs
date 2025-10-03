use chrono::NaiveDate;
use lifestats_domain::shared::ids::StatisticId;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;

use crate::repositories::postgres_log_repository::PostgresLogRepository;
use anyhow::Result;
use lifestats_domain::statistic::ports::log_repository::StatisticLogRepository;
use lifestats_domain::value_objects::date_range::DateRange;
use lifestats_domain::value_objects::dated_value::DatedValue;

pub struct TypedPostgresLogRepository<T> {
    untyped_repository: Arc<PostgresLogRepository>,
}

impl<T> StatisticLogRepository for TypedPostgresLogRepository<T>
where
    T: Serialize + DeserializeOwned,
{
    type LogValueType = T;

    fn get(&self, statistic_id: StatisticId, date: NaiveDate) -> Result<Option<T>> {
        let untyped_value = self.untyped_repository.get(statistic_id, date)?;

        untyped_value
            .map(|v| serde_json::from_value(v))
            .transpose()
            .into()
    }

    fn get_range(
        &self,
        statistic_id: StatisticId,
        date_range: DateRange,
    ) -> Result<Vec<DatedValue<T>>> {
        let untyped_range = self
            .untyped_repository
            .get_range(statistic_id, date_range)?;
        untyped_range
            .into_iter()
            .map(|dated_value| {
                let value = serde_json::from_value(dated_value.value)?;
                Ok(DatedValue {
                    date: dated_value.date,
                    value,
                })
            })
            .collect()
    }

    fn save(&self, statistic_id: StatisticId, date: NaiveDate, value: T) -> Result<()> {
        let serialized = serde_json::to_value(value)?;
        self.untyped_repository.save(statistic_id, date, serialized)
    }
}
