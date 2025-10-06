use chrono::NaiveDate;

// TODO: Do we need the statistic_id here?
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct LogData<T> {
    pub date: NaiveDate,
    pub value: Option<T>,
}

impl<T> LogData<T> {
    pub fn new(date: NaiveDate, value: Option<T>) -> Self {
        Self { date, value }
    }
}
