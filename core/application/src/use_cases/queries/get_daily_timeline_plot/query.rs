use crate::cqrs::Query;
use chrono::NaiveDate;

/// Query to retrieve a rendered timeline plot for a specific date.
pub struct GetDailyTimelinePlotQuery {
    pub date: NaiveDate,
}

impl Query for GetDailyTimelinePlotQuery {
    type Result = Vec<u8>;
}
