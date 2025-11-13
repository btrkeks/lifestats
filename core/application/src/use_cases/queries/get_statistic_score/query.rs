use crate::cqrs::Query;
use chrono::NaiveDate;
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::shared::percentage::Percentage;

/// Query to compute the mean daily goal achievement percentage for a statistic
/// over a given date range.
pub struct GetStatisticScoreQuery {
    pub statistic_id: StatisticId,
    pub date_range: DateRange,
}

/// Result of a statistic score query.
///
/// - `mean_percentage_achieved` is `None` if there were no days with a goal in the range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatisticScoreResult {
    pub statistic_id: StatisticId,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub mean_percentage_achieved: Option<Percentage>,
    /// Number of days in the range that had a goal and contributed to the score
    pub days_with_goal: u32,
}

impl Query for GetStatisticScoreQuery {
    type Result = StatisticScoreResult;
}
