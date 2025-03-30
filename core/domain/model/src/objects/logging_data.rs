use chrono::NaiveDate;
use transfer_objects::statistic_id::StatisticId;

// TODO: Do we need the statistic_id here?
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct StatisticData<T> {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub value: Option<T>,
}

impl<T> StatisticData<T> {
    pub fn new(statistic_id: StatisticId, date: NaiveDate, value: Option<T>) -> Self {
        Self {
            statistic_id,
            date,
            value,
        }
    }
}