use crate::shared::date_range::DateRange;
use crate::shared::dated_value::DatedValue;
use crate::shared::ids::StatisticId;
use anyhow::Result;
use chrono::NaiveDate;

pub trait LoggingDataRepositoryTrait {
    type StoredType;

    fn get(
        &self,
        statistic_id: StatisticId,
        date: NaiveDate,
    ) -> Result<Option<Self::StoredType>>;

    fn get_in_closed_range(
        &self,
        statistic_id: StatisticId,
        date_range: DateRange,
    ) -> Result<Vec<DatedValue<Self::StoredType>>>;

    fn insert_or_update(
        &self,
        statistic_id: StatisticId,
        date: NaiveDate,
        new_value: Self::StoredType,
    ) -> Result<()>;
}
