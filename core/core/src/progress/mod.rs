pub mod checkpoint;
pub mod error;
pub mod item;
pub mod ports;
pub mod unit;

pub use checkpoint::{NewProgressCheckpoint, ProgressCheckpoint};
pub use item::{NewProgressItem, ProgressItem, ProgressMetadata, ProgressTarget};
pub use unit::ProgressUnit;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::{NaiveDate, TimeZone, Utc};

    use super::*;
    use crate::shared::ids::ProgressItemId;
    use crate::statistic::error::DomainError;

    #[test]
    fn creates_valid_progress_item() {
        let metadata =
            ProgressMetadata::new("Book".to_string(), "Reading".to_string(), HashMap::new())
                .unwrap();
        let target = ProgressTarget::new(304, ProgressUnit::Pages).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 5, 2, 12, 0, 0).unwrap();

        let item = ProgressItem::new(ProgressItemId(1), metadata, target, now, now).unwrap();

        assert_eq!(item.id(), ProgressItemId(1));
        assert_eq!(item.metadata().name(), "Book");
        assert_eq!(item.target().value(), 304);
    }

    #[test]
    fn rejects_invalid_item_inputs() {
        assert!(matches!(
            ProgressMetadata::new(" ".to_string(), "Reading".to_string(), HashMap::new()),
            Err(DomainError::ValidationError(_))
        ));
        assert!(matches!(
            ProgressMetadata::new("Book".to_string(), "".to_string(), HashMap::new()),
            Err(DomainError::ValidationError(_))
        ));
        assert!(matches!(
            ProgressTarget::new(0, ProgressUnit::Pages),
            Err(DomainError::ValidationError(_))
        ));
        assert!(matches!(
            ProgressTarget::new(
                10,
                ProgressUnit::Custom {
                    label: " ".to_string()
                }
            ),
            Err(DomainError::ValidationError(_))
        ));
    }

    #[test]
    fn creates_checkpoint_and_normalizes_note() {
        let now = Utc.with_ymd_and_hms(2026, 5, 2, 20, 0, 0).unwrap();
        let checkpoint = ProgressCheckpoint::new(
            ProgressItemId(1),
            now,
            NaiveDate::from_ymd_opt(2026, 5, 2).unwrap(),
            73,
            Some(" ebook ".to_string()),
            Some(" event-1 ".to_string()),
            Some("   ".to_string()),
            now,
        )
        .unwrap();

        assert_eq!(checkpoint.position(), 73);
        assert_eq!(checkpoint.source(), Some("ebook"));
        assert_eq!(checkpoint.source_event_id(), Some("event-1"));
        assert_eq!(checkpoint.note(), None);
    }

    #[test]
    fn rejects_invalid_checkpoint_inputs() {
        let now = Utc.with_ymd_and_hms(2026, 5, 2, 20, 0, 0).unwrap();
        assert!(ProgressCheckpoint::new(
            ProgressItemId(1),
            now,
            NaiveDate::from_ymd_opt(2026, 5, 2).unwrap(),
            -1,
            None,
            None,
            None,
            now,
        )
        .is_err());

        assert!(ProgressCheckpoint::new(
            ProgressItemId(1),
            now,
            NaiveDate::from_ymd_opt(2026, 5, 2).unwrap(),
            1,
            None,
            Some("event-1".to_string()),
            None,
            now,
        )
        .is_err());
    }
}
