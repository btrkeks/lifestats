use crate::error::ApplicationResult;
use chrono::NaiveDate;
use lifestats_core::shared::ids::StatisticId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EbookSyncEvent {
    pub event_id: String,
    pub source_name: String,
    pub metric: String,
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub delta: i64,
    pub book_id: Option<String>,
    pub book_title: String,
    pub confirmed_position_after: i64,
}

pub trait EbookEventStore: Send + Sync {
    /// Records an ebook sync event exactly once and applies the associated delta.
    ///
    /// Returns `true` when the event was newly accepted and applied.
    /// Returns `false` when the event was already seen before.
    fn record_event(&self, event: &EbookSyncEvent) -> ApplicationResult<bool>;
}
