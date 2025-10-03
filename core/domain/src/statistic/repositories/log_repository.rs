use anyhow::Result;
use crate::shared::dated_value::DatedValue;
use chrono::NaiveDate;
use crate::shared::ids::StatisticId;
use crate::shared::date_range::DateRange;

pub trait StatisticLogRepository {
    type LogValueType;
    fn get(&self, statistic_id: StatisticId, date: NaiveDate)
        -> Result<Option<Self::LogValueType>>;
    fn get_range(
        &self,
        statistic_id: StatisticId,
        date_range: DateRange
    ) -> Result<Vec<DatedValue<Self::LogValueType>>>;
    fn save(
        &self,
        statistic_id: StatisticId,
        date: NaiveDate,
        value: Self::LogValueType,
    ) -> Result<()>;
}
