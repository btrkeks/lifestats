use crate::cqrs::Query;
use chrono::NaiveDate;
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::ids::StatisticId;

pub struct GetTimeSpansTotalQuery {
    pub statistic_id: StatisticId,
    pub date_range: DateRange,
}

impl Query for GetTimeSpansTotalQuery {
    type Result = TimeSpansTotalResult;
}

#[derive(Debug)]
pub struct TimeSpansTotalResult {
    pub statistic_id: StatisticId,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub total_seconds: i64,
    pub days_with_data: usize,
}
