use chrono::NaiveDate;
use crate::statistic::error::Result;

pub trait StatisticLogRepository {
    type LogValueType;
    fn get(&self, date: NaiveDate) -> Result<Option<Self::LogValueType>>;
    fn set(&self, date: NaiveDate, value: Self::LogValueType) -> Result<()>;
}