use chrono::NaiveDate;
use lifestats_core::shared::time_frame_list::TimeFrame;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub struct DailySummaryResponse {
    pub date: NaiveDate,
    pub category_percentages: HashMap<String, f64>,
    pub timeline_events: Vec<TimelineEvent>,
}

#[derive(Clone, Serialize)]
pub struct TimelineEvent {
    pub timeframe: TimeFrame,
    pub label: String,
}
