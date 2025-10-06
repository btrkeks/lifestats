use chrono::NaiveDate;

pub struct DatedValue<T> {
    pub date: NaiveDate,
    pub value: T,
}

impl<T> DatedValue<T> {
    pub fn new(date: NaiveDate, value: T) -> Self {
        Self { date, value }
    }
}
