use anyhow::Result;
use crate::shared::dated_value::DatedValue;
use chrono::NaiveDate;
use crate::shared::ids::StatisticId;
use crate::shared::date_range::DateRange;

pub trait GoalRepository {
    type GoalType;
    fn get(&self, statistic_id: StatisticId, date: NaiveDate) -> Result<Option<Self::GoalType>>;
    fn get_range(
        &self,
        statistic_id: StatisticId,
        date_range: DateRange,
    ) -> Result<Vec<DatedValue<Self::GoalType>>>;
    fn save(&self, statistic_id: StatisticId, date: NaiveDate, value: Self::GoalType)
        -> Result<()>;
}
