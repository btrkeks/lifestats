use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use lifestats_core::progress::ports::progress_repository::{
    ProgressCheckpointRange, ProgressRepository,
};
use lifestats_core::shared::ports::error::RepositoryError;

use super::query::{GetProgressSummaryQuery, ProgressSummary};

pub struct GetProgressSummaryQueryHandler {
    repository: Arc<dyn ProgressRepository + Send + Sync>,
}

impl GetProgressSummaryQueryHandler {
    pub fn new(repository: Arc<dyn ProgressRepository + Send + Sync>) -> Self {
        Self { repository }
    }
}

impl QueryHandler<GetProgressSummaryQuery> for GetProgressSummaryQueryHandler {
    fn handle(&self, query: GetProgressSummaryQuery) -> ApplicationResult<ProgressSummary> {
        let item = self
            .repository
            .find_item_by_id(query.item_id)?
            .ok_or(RepositoryError::NotFound)?;
        let latest_checkpoint = self.repository.find_latest_checkpoint(query.item_id)?;
        let checkpoints = self
            .repository
            .find_checkpoints(query.item_id, ProgressCheckpointRange::default())?;

        let latest_position = latest_checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.position());
        let furthest_position = checkpoints
            .iter()
            .map(|checkpoint| checkpoint.position())
            .max();
        let target = item.target().value() as f64;
        let percent_latest =
            latest_position.map(|position| clamp_percent(position as f64 / target));
        let percent_furthest =
            furthest_position.map(|position| clamp_percent(position as f64 / target));
        let is_complete = furthest_position
            .map(|position| position >= item.target().value())
            .unwrap_or(false);

        Ok(ProgressSummary {
            item,
            latest_checkpoint,
            latest_position,
            furthest_position,
            percent_latest,
            percent_furthest,
            is_complete,
        })
    }
}

fn clamp_percent(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use lifestats_core::progress::ports::progress_repository::MockProgressRepository;
    use lifestats_core::progress::{
        ProgressCheckpoint, ProgressItem, ProgressMetadata, ProgressTarget, ProgressUnit,
    };
    use lifestats_core::shared::ids::{ProgressCheckpointId, ProgressItemId};
    use std::collections::HashMap;

    fn item() -> ProgressItem {
        let now = Utc.with_ymd_and_hms(2026, 5, 1, 12, 0, 0).unwrap();
        ProgressItem::new(
            ProgressItemId(1),
            ProgressMetadata::new("Book".to_string(), "Reading".to_string(), HashMap::new())
                .unwrap(),
            ProgressTarget::new(100, ProgressUnit::Pages).unwrap(),
            now,
            now,
        )
        .unwrap()
    }

    fn checkpoint(id: u32, day: u32, position: i64) -> ProgressCheckpoint {
        let recorded_at = Utc.with_ymd_and_hms(2026, 5, day, 20, 0, 0).unwrap();
        ProgressCheckpoint::new(
            ProgressItemId(1),
            recorded_at,
            NaiveDate::from_ymd_opt(2026, 5, day).unwrap(),
            position,
            None,
            None,
            None,
            recorded_at,
        )
        .unwrap()
        .with_id(ProgressCheckpointId(id))
    }

    #[test]
    fn summary_distinguishes_latest_and_furthest_after_backward_movement() {
        let mut repo = MockProgressRepository::new();
        let item = item();
        let latest = checkpoint(3, 3, 40);
        let checkpoints = vec![checkpoint(1, 1, 30), checkpoint(2, 2, 70), latest.clone()];

        repo.expect_find_item_by_id()
            .times(1)
            .returning(move |_| Ok(Some(item.clone())));
        repo.expect_find_latest_checkpoint()
            .times(1)
            .returning(move |_| Ok(Some(latest.clone())));
        repo.expect_find_checkpoints()
            .times(1)
            .returning(move |_, _| Ok(checkpoints.clone()));

        let handler = GetProgressSummaryQueryHandler::new(Arc::new(repo));
        let summary = handler
            .handle(GetProgressSummaryQuery {
                item_id: ProgressItemId(1),
            })
            .unwrap();

        assert_eq!(summary.latest_position, Some(40));
        assert_eq!(summary.furthest_position, Some(70));
        assert_eq!(summary.percent_latest, Some(0.4));
        assert_eq!(summary.percent_furthest, Some(0.7));
        assert!(!summary.is_complete);
    }
}
