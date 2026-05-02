use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use lifestats_core::progress::ports::progress_repository::{
    ProgressRepository, SaveCheckpointResult,
};
use lifestats_core::progress::{
    NewProgressCheckpoint, NewProgressItem, ProgressItem, ProgressMetadata, ProgressTarget,
    ProgressUnit,
};
use lifestats_core::shared::ids::ProgressItemId;

use crate::error::{ApplicationError, ApplicationResult};

const EBOOK_SOURCE: &str = "ebook";
const EBOOK_PROGRESS_TYPE_BOOK: &str = "book";
const EBOOK_CATEGORY: &str = "読書";

#[cfg_attr(test, mockall::automock)]
pub trait EbookProgressServiceTrait: Send + Sync {
    fn ensure_and_log_checkpoint(
        &self,
        request: EbookProgressSyncRequest,
    ) -> ApplicationResult<EbookProgressSyncResult>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EbookProgressSyncRequest {
    pub event_id: String,
    pub book_id: String,
    pub book_title: String,
    pub effective_date: NaiveDate,
    pub recorded_at: DateTime<Utc>,
    pub confirmed_position_after: i64,
    pub book_target_characters: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EbookProgressSyncResult {
    pub item_id: ProgressItemId,
    pub checkpoint_result: SaveCheckpointResult,
}

pub struct EbookProgressService {
    repository: Arc<dyn ProgressRepository + Send + Sync>,
}

impl EbookProgressService {
    pub fn new(repository: Arc<dyn ProgressRepository + Send + Sync>) -> Self {
        Self { repository }
    }

    fn lookup_tags(book_id: &str) -> HashMap<String, String> {
        HashMap::from([
            ("source".to_string(), EBOOK_SOURCE.to_string()),
            (
                "progress_type".to_string(),
                EBOOK_PROGRESS_TYPE_BOOK.to_string(),
            ),
            ("reader_data_id".to_string(), book_id.to_string()),
        ])
    }

    fn item_tags(book_id: &str, book_title: &str) -> HashMap<String, String> {
        let mut tags = Self::lookup_tags(book_id);
        tags.insert("book_title".to_string(), book_title.to_string());
        tags
    }

    fn validate(request: &EbookProgressSyncRequest) -> ApplicationResult<()> {
        if request.event_id.trim().is_empty() {
            return Err(ApplicationError::InvalidInput(
                "ebook event_id must not be empty".to_string(),
            ));
        }

        if request.book_id.trim().is_empty() {
            return Err(ApplicationError::InvalidInput(
                "ebook book_id must not be empty".to_string(),
            ));
        }

        if request.book_title.trim().is_empty() {
            return Err(ApplicationError::InvalidInput(
                "ebook book_title must not be empty".to_string(),
            ));
        }

        if request.confirmed_position_after < 0 {
            return Err(ApplicationError::InvalidInput(
                "ebook confirmed_position_after must be non-negative".to_string(),
            ));
        }

        if request.book_target_characters <= 0 {
            return Err(ApplicationError::InvalidInput(
                "ebook book_target_characters must be positive".to_string(),
            ));
        }

        Ok(())
    }

    fn ensure_item(
        &self,
        request: &EbookProgressSyncRequest,
        target_value: i64,
    ) -> ApplicationResult<ProgressItem> {
        let lookup_tags = Self::lookup_tags(request.book_id.trim());
        let existing = self.repository.find_items_by_tags(&lookup_tags)?;

        if let Some(item) = existing.into_iter().next() {
            if item.target().value() == target_value {
                return Ok(item);
            }

            let updated = item.with_target_value(target_value, request.recorded_at)?;
            self.repository.update_item(updated.clone())?;
            return Ok(updated);
        }

        let metadata = ProgressMetadata::new(
            request.book_title.trim().to_string(),
            EBOOK_CATEGORY.to_string(),
            Self::item_tags(request.book_id.trim(), request.book_title.trim()),
        )?;
        let target = ProgressTarget::new(target_value, ProgressUnit::Characters)?;
        let item = NewProgressItem::new(metadata, target, request.recorded_at);

        Ok(self.repository.create_item(item)?)
    }
}

impl EbookProgressServiceTrait for EbookProgressService {
    fn ensure_and_log_checkpoint(
        &self,
        request: EbookProgressSyncRequest,
    ) -> ApplicationResult<EbookProgressSyncResult> {
        Self::validate(&request)?;

        let target_value = std::cmp::max(
            request.book_target_characters,
            request.confirmed_position_after,
        );
        let item = self.ensure_item(&request, target_value)?;

        let checkpoint = NewProgressCheckpoint::new(
            item.id(),
            request.recorded_at,
            request.effective_date,
            request.confirmed_position_after,
            Some(EBOOK_SOURCE.to_string()),
            Some(request.event_id.clone()),
            None,
            request.recorded_at,
        )?;
        let checkpoint_result = self.repository.save_checkpoint(checkpoint)?;

        Ok(EbookProgressSyncResult {
            item_id: item.id(),
            checkpoint_result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use lifestats_core::progress::ports::progress_repository::MockProgressRepository;
    use lifestats_core::progress::{ProgressCheckpoint, ProgressMetadata, ProgressTarget};

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 2, 20, 15, 0).unwrap()
    }

    fn request() -> EbookProgressSyncRequest {
        EbookProgressSyncRequest {
            event_id: "evt-1".to_string(),
            book_id: "7".to_string(),
            book_title: "Book".to_string(),
            effective_date: NaiveDate::from_ymd_opt(2026, 5, 2).unwrap(),
            recorded_at: now(),
            confirmed_position_after: 420,
            book_target_characters: 1_000,
        }
    }

    fn item(id: u32, target_value: i64) -> ProgressItem {
        ProgressItem::new(
            ProgressItemId(id),
            ProgressMetadata::new(
                "Book".to_string(),
                EBOOK_CATEGORY.to_string(),
                HashMap::new(),
            )
            .unwrap(),
            ProgressTarget::new(target_value, ProgressUnit::Characters).unwrap(),
            now(),
            now(),
        )
        .unwrap()
    }

    fn checkpoint(position: i64) -> ProgressCheckpoint {
        ProgressCheckpoint::new(
            ProgressItemId(7),
            now(),
            NaiveDate::from_ymd_opt(2026, 5, 2).unwrap(),
            position,
            Some(EBOOK_SOURCE.to_string()),
            Some("evt-1".to_string()),
            None,
            now(),
        )
        .unwrap()
    }

    #[test]
    fn creates_item_and_logs_checkpoint_when_missing() {
        let mut repo = MockProgressRepository::new();

        repo.expect_find_items_by_tags()
            .times(1)
            .withf(|tags| {
                tags.get("source").map(String::as_str) == Some(EBOOK_SOURCE)
                    && tags.get("progress_type").map(String::as_str) == Some("book")
                    && tags.get("reader_data_id").map(String::as_str) == Some("7")
            })
            .returning(|_| Ok(vec![]));

        repo.expect_create_item()
            .times(1)
            .withf(|new_item| {
                new_item.metadata().name() == "Book"
                    && new_item.metadata().category() == EBOOK_CATEGORY
                    && new_item.target().value() == 1_000
                    && matches!(new_item.target().unit(), ProgressUnit::Characters)
                    && new_item
                        .metadata()
                        .tags()
                        .get("reader_data_id")
                        .map(String::as_str)
                        == Some("7")
            })
            .returning(|_| Ok(item(7, 1_000)));

        repo.expect_update_item().times(0);

        repo.expect_save_checkpoint()
            .times(1)
            .withf(|new_checkpoint| {
                let checkpoint = new_checkpoint.checkpoint();
                checkpoint.item_id() == ProgressItemId(7)
                    && checkpoint.position() == 420
                    && checkpoint.source() == Some(EBOOK_SOURCE)
                    && checkpoint.source_event_id() == Some("evt-1")
            })
            .returning(|_| Ok(SaveCheckpointResult::Inserted(checkpoint(420))));

        let service = EbookProgressService::new(Arc::new(repo));
        let result = service.ensure_and_log_checkpoint(request()).unwrap();

        assert_eq!(result.item_id, ProgressItemId(7));
        assert!(matches!(
            result.checkpoint_result,
            SaveCheckpointResult::Inserted(_)
        ));
    }

    #[test]
    fn reuses_existing_item() {
        let mut repo = MockProgressRepository::new();

        repo.expect_find_items_by_tags()
            .times(1)
            .returning(|_| Ok(vec![item(9, 1_000)]));
        repo.expect_create_item().times(0);
        repo.expect_update_item().times(0);
        repo.expect_save_checkpoint()
            .times(1)
            .returning(|_| Ok(SaveCheckpointResult::Inserted(checkpoint(420))));

        let service = EbookProgressService::new(Arc::new(repo));
        let result = service.ensure_and_log_checkpoint(request()).unwrap();

        assert_eq!(result.item_id, ProgressItemId(9));
    }

    #[test]
    fn updates_target_when_total_characters_change() {
        let mut repo = MockProgressRepository::new();

        repo.expect_find_items_by_tags()
            .times(1)
            .returning(|_| Ok(vec![item(9, 900)]));
        repo.expect_create_item().times(0);
        repo.expect_update_item()
            .times(1)
            .withf(|item| item.id() == ProgressItemId(9) && item.target().value() == 1_000)
            .returning(|_| Ok(()));
        repo.expect_save_checkpoint()
            .times(1)
            .returning(|_| Ok(SaveCheckpointResult::Inserted(checkpoint(420))));

        let service = EbookProgressService::new(Arc::new(repo));
        let result = service.ensure_and_log_checkpoint(request()).unwrap();

        assert_eq!(result.item_id, ProgressItemId(9));
    }

    #[test]
    fn target_never_drops_below_confirmed_position() {
        let mut repo = MockProgressRepository::new();

        repo.expect_find_items_by_tags()
            .times(1)
            .returning(|_| Ok(vec![]));
        repo.expect_create_item()
            .times(1)
            .withf(|new_item| new_item.target().value() == 420)
            .returning(|_| Ok(item(7, 420)));
        repo.expect_save_checkpoint()
            .times(1)
            .returning(|_| Ok(SaveCheckpointResult::Inserted(checkpoint(420))));

        let service = EbookProgressService::new(Arc::new(repo));
        let result = service
            .ensure_and_log_checkpoint(EbookProgressSyncRequest {
                book_target_characters: 100,
                ..request()
            })
            .unwrap();

        assert_eq!(result.item_id, ProgressItemId(7));
    }

    #[test]
    fn duplicate_checkpoint_result_is_preserved() {
        let mut repo = MockProgressRepository::new();

        repo.expect_find_items_by_tags()
            .times(1)
            .returning(|_| Ok(vec![item(7, 1_000)]));
        repo.expect_save_checkpoint()
            .times(1)
            .returning(|_| Ok(SaveCheckpointResult::Duplicate(checkpoint(420))));

        let service = EbookProgressService::new(Arc::new(repo));
        let result = service.ensure_and_log_checkpoint(request()).unwrap();

        assert!(matches!(
            result.checkpoint_result,
            SaveCheckpointResult::Duplicate(_)
        ));
    }
}
