use crate::cqrs::Query;
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::ids::StatisticId;

pub struct GetIntegerAmountTotalQuery {
    pub statistic_id: StatisticId,
    pub date_range: DateRange,
}

impl Query for GetIntegerAmountTotalQuery {
    type Result = IntegerAmountTotalResult;
}

#[derive(Debug)]
pub struct IntegerAmountTotalResult {
    pub total: i64,
}
