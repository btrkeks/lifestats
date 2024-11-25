use chrono::NaiveDate;
use crate::objects::statistic_metadata::StatisticId;
use anyhow::Result;

pub trait StatisticDataRepository {
    fn get_in_closed_range(&self, id: StatisticId, start_date: NaiveDate, end_date: NaiveDate) -> Result<Vec<i64>>;
}