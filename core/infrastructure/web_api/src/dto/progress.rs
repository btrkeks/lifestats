use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use lifestats_application::use_cases::queries::get_progress_summary::ProgressSummary;
use lifestats_core::progress::{ProgressCheckpoint, ProgressItem, ProgressUnit};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateProgressItemRequest {
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(length(min = 1))]
    pub category: String,
    #[validate(range(min = 1))]
    pub target_value: i64,
    pub unit: ProgressUnit,
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProgressTargetRequest {
    #[validate(range(min = 1))]
    pub target_value: i64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LogProgressCheckpointRequest {
    pub recorded_at: Option<DateTime<Utc>>,
    pub effective_date: NaiveDate,
    #[validate(range(min = 0))]
    pub position: i64,
    pub source: Option<String>,
    pub source_event_id: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProgressCheckpointsParams {
    pub recorded_at_start: Option<DateTime<Utc>>,
    pub recorded_at_end: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ProgressDailyDeltasParams {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProgressItemResponse {
    pub id: u32,
    pub name: String,
    pub category: String,
    pub target_value: i64,
    pub unit: ProgressUnit,
    pub tags: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&ProgressItem> for ProgressItemResponse {
    fn from(item: &ProgressItem) -> Self {
        Self {
            id: item.id().0,
            name: item.metadata().name().to_string(),
            category: item.metadata().category().to_string(),
            target_value: item.target().value(),
            unit: item.target().unit().clone(),
            tags: item.metadata().tags().clone(),
            created_at: item.created_at(),
            updated_at: item.updated_at(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProgressCheckpointResponse {
    pub id: Option<u32>,
    pub item_id: u32,
    pub recorded_at: DateTime<Utc>,
    pub effective_date: NaiveDate,
    pub position: i64,
    pub source: Option<String>,
    pub source_event_id: Option<String>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<&ProgressCheckpoint> for ProgressCheckpointResponse {
    fn from(checkpoint: &ProgressCheckpoint) -> Self {
        Self {
            id: checkpoint.id().map(|id| id.0),
            item_id: checkpoint.item_id().0,
            recorded_at: checkpoint.recorded_at(),
            effective_date: checkpoint.effective_date(),
            position: checkpoint.position(),
            source: checkpoint.source().map(str::to_string),
            source_event_id: checkpoint.source_event_id().map(str::to_string),
            note: checkpoint.note().map(str::to_string),
            created_at: checkpoint.created_at(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LogProgressCheckpointResponse {
    pub status: String,
    pub checkpoint: ProgressCheckpointResponse,
}

#[derive(Debug, Serialize)]
pub struct ProgressSummaryResponse {
    pub item: ProgressItemResponse,
    pub latest_checkpoint: Option<ProgressCheckpointResponse>,
    pub latest_position: Option<i64>,
    pub furthest_position: Option<i64>,
    pub percent_latest: Option<f64>,
    pub percent_furthest: Option<f64>,
    pub is_complete: bool,
}

impl From<ProgressSummary> for ProgressSummaryResponse {
    fn from(summary: ProgressSummary) -> Self {
        Self {
            item: ProgressItemResponse::from(&summary.item),
            latest_checkpoint: summary
                .latest_checkpoint
                .as_ref()
                .map(ProgressCheckpointResponse::from),
            latest_position: summary.latest_position,
            furthest_position: summary.furthest_position,
            percent_latest: summary.percent_latest,
            percent_furthest: summary.percent_furthest,
            is_complete: summary.is_complete,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProgressDailyDeltaResponse {
    pub date: NaiveDate,
    pub value: i64,
}
