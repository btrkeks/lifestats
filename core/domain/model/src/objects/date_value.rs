use chrono::NaiveDate;

pub struct DateValue<T> {
    pub date: NaiveDate,
    pub value: T,
}

impl<T> DateValue<T> {
    pub fn new(date: NaiveDate, value: T) -> Self {
        Self { date, value }
    }
}
