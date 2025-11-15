pub mod get_all_metadata;
pub mod get_all_statistics;
pub mod get_all_timeline_events;
pub mod get_consistency_score;
pub mod get_daily_goal;
pub mod get_daily_summary;
pub mod get_daily_timeline_plot;
pub mod get_default_goal;
pub mod get_goal_performance;
pub mod get_loggable_metadata;
pub mod get_metadata;
pub mod get_plot_data;
pub mod get_statistic_by_id;
pub mod get_statistic_score;
pub mod get_streak;
pub mod get_time_spans_metadata;
pub mod get_time_spans_total;

pub use get_all_metadata::{GetAllMetadataQuery, GetAllMetadataQueryHandler};
pub use get_all_statistics::{GetAllStatisticsQuery, GetAllStatisticsQueryHandler};
pub use get_all_timeline_events::{GetAllTimelineEventsQuery, GetAllTimelineEventsQueryHandler};
pub use get_consistency_score::{
    ConsistencyScoreResult, GetConsistencyScoreQuery, GetConsistencyScoreQueryHandler,
};
pub use get_daily_goal::{GetDailyGoalQuery, GetDailyGoalQueryHandler};
pub use get_daily_summary::{GetDailySummaryQuery, GetDailySummaryQueryHandler};
pub use get_daily_timeline_plot::{GetDailyTimelinePlotQuery, GetDailyTimelinePlotQueryHandler};
pub use get_default_goal::{GetDefaultGoalQuery, GetDefaultGoalQueryHandler};
pub use get_goal_performance::{GetGoalPerformanceQuery, GetGoalPerformanceQueryHandler};
pub use get_loggable_metadata::{GetLoggableMetadataQuery, GetLoggableMetadataQueryHandler};
pub use get_metadata::{GetMetadataQuery, GetMetadataQueryHandler};
pub use get_plot_data::{GetPlotDataQuery, GetPlotDataQueryHandler};
pub use get_statistic_by_id::{GetStatisticByIdQuery, GetStatisticByIdQueryHandler};
pub use get_statistic_score::{
    GetStatisticScoreQuery, GetStatisticScoreQueryHandler, StatisticScoreResult,
};
pub use get_streak::{GetStreakQuery, GetStreakQueryHandler};
pub use get_time_spans_metadata::{GetTimeSpansMetadataQuery, GetTimeSpansMetadataQueryHandler};
pub use get_time_spans_total::{GetTimeSpansTotalQuery, GetTimeSpansTotalQueryHandler};
