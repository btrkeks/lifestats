use crate::{cqrs::Query, dto::DailySummaryResponse};
use chrono::NaiveDate;

/// Query to retrieve a daily summary for a specific date.
pub struct GetDailySummaryQuery {
    pub date: NaiveDate,
}

impl Query for GetDailySummaryQuery {
    type Result = DailySummaryResponse;
}
