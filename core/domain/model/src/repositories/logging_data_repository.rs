use crate::statistic::objects::logging_data::DateValue;
use anyhow::Result;
use chrono::NaiveDate;
use serde::de::DeserializeOwned;
use serde::Serialize;
use transfer_objects::statistic_id::StatisticId;

pub trait LoggingDataRepositoryTrait: Sync + Send {
    type StoredType: Serialize + DeserializeOwned;

    fn get_in_closed_range(
        &self,
        id: StatisticId,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<DateValue<Self::StoredType>>>;

    fn get(&self, id: StatisticId, date: NaiveDate) -> Result<Option<Self::StoredType>>;

    fn insert_or_update(
        &self,
        id: StatisticId,
        date: NaiveDate,
        new_value: Self::StoredType,
    ) -> Result<()>;
}
