use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::dated_value::DatedValue;
use lifestats_core::shared::ids::ProgressItemId;

use crate::cqrs::Query;

pub struct GetProgressDailyDeltasQuery {
    pub item_id: ProgressItemId,
    pub date_range: DateRange,
}

impl Query for GetProgressDailyDeltasQuery {
    type Result = Vec<DatedValue<i64>>;
}
