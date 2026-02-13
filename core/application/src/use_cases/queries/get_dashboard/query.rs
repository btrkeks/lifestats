use chrono::NaiveDate;
use lifestats_core::shared::ids::StatisticId;

use crate::cqrs::Query;
use crate::dto::dashboard::DashboardResponseDto;

/// Query to fetch dashboard data for multiple statistics.
///
/// Returns for each statistic:
/// - Today's logged value
/// - 7-day rolling average
/// - Goal performance over the specified window
#[derive(Debug, Clone)]
pub struct GetDashboardQuery {
    /// Statistics to include in the dashboard.
    pub statistic_ids: Vec<StatisticId>,
    /// Reference date (typically "today" from client).
    pub today: NaiveDate,
    /// Number of days for performance window.
    pub performance_window_days: u32,
}

impl Query for GetDashboardQuery {
    type Result = DashboardResponseDto;
}
