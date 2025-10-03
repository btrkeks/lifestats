use chrono::{DateTime, NaiveDate, Utc};
use lifestats_domain::shared::ids::StatisticId;
use lifestats_domain::statistic::value_objects::statistic_metadata::StatisticMetadata;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum StatisticKind {
    TimeSpan,
    Habit,
    IntegerAmount,
    PointInTime,
    DerivedSum,
    DerivedTimeDifference,
}

#[derive(Serialize, Deserialize)]
pub struct StatisticDTO {
    id: StatisticId,
    kind: StatisticKind,
    metadata: StatisticMetadata,

    // For derived statistics
    source_statistic_ids: Vec<StatisticId>,

    // Configuration JSON serialized to string
    config: String,
}

#[derive(Serialize, Deserialize)]
pub struct LogEntryDTO {
    statistic_id: StatisticId,
    date: NaiveDate,
    value_type: String, // e.g., "TimeFrameList", "bool", "i64"
    value_json: String, // JSON serialized value
    timestamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct GoalDTO {
    statistic_id: StatisticId,
    date: NaiveDate,
    goal_type: String, // e.g., "TimeSpansGoal", "HabitGoal"
    goal_json: String, // JSON serialized goal
}
