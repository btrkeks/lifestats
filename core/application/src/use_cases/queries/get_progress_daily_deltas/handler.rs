use std::collections::BTreeMap;
use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use lifestats_core::progress::ports::progress_repository::{
    ProgressCheckpointRange, ProgressRepository,
};
use lifestats_core::shared::dated_value::DatedValue;
use lifestats_core::shared::ports::error::RepositoryError;

use super::query::GetProgressDailyDeltasQuery;

pub struct GetProgressDailyDeltasQueryHandler {
    repository: Arc<dyn ProgressRepository + Send + Sync>,
}

impl GetProgressDailyDeltasQueryHandler {
    pub fn new(repository: Arc<dyn ProgressRepository + Send + Sync>) -> Self {
        Self { repository }
    }
}

impl QueryHandler<GetProgressDailyDeltasQuery> for GetProgressDailyDeltasQueryHandler {
    fn handle(
        &self,
        query: GetProgressDailyDeltasQuery,
    ) -> ApplicationResult<Vec<DatedValue<i64>>> {
        self.repository
            .find_item_by_id(query.item_id)?
            .ok_or(RepositoryError::NotFound)?;

        let recorded_at_end = query
            .date_range
            .end()
            .and_hms_opt(23, 59, 59)
            .expect("valid end-of-day")
            .and_utc();

        let checkpoints = self.repository.find_checkpoints(
            query.item_id,
            ProgressCheckpointRange {
                recorded_at_start: None,
                recorded_at_end: Some(recorded_at_end),
            },
        )?;

        let mut frontier = 0;
        let mut deltas = BTreeMap::new();
        for checkpoint in checkpoints {
            let delta = (checkpoint.position() - frontier).max(0);
            frontier = frontier.max(checkpoint.position());

            if delta > 0
                && checkpoint.effective_date() >= query.date_range.start()
                && checkpoint.effective_date() <= query.date_range.end()
            {
                *deltas.entry(checkpoint.effective_date()).or_insert(0) += delta;
            }
        }

        Ok(deltas
            .into_iter()
            .map(|(date, value)| DatedValue::new(date, value))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use lifestats_core::progress::ports::progress_repository::MockProgressRepository;
    use lifestats_core::progress::{
        ProgressCheckpoint, ProgressItem, ProgressMetadata, ProgressTarget, ProgressUnit,
    };
    use lifestats_core::shared::date_range::DateRange;
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

    fn checkpoint(
        id: u32,
        recorded_day: u32,
        effective_day: u32,
        position: i64,
    ) -> ProgressCheckpoint {
        let recorded_at = Utc
            .with_ymd_and_hms(2026, 5, recorded_day, 20, 0, 0)
            .unwrap();
        ProgressCheckpoint::new(
            ProgressItemId(1),
            recorded_at,
            NaiveDate::from_ymd_opt(2026, 5, effective_day).unwrap(),
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
    fn daily_deltas_use_furthest_frontier_and_do_not_subtract() {
        let checkpoints = vec![
            checkpoint(1, 1, 1, 40),
            checkpoint(2, 2, 2, 30),
            checkpoint(3, 3, 3, 45),
            checkpoint(4, 4, 4, 60),
        ];
        let mut repo = MockProgressRepository::new();
        repo.expect_find_item_by_id()
            .times(1)
            .returning(|_| Ok(Some(item())));
        repo.expect_find_checkpoints()
            .times(1)
            .returning(move |_, _| Ok(checkpoints.clone()));

        let handler = GetProgressDailyDeltasQueryHandler::new(Arc::new(repo));
        let result = handler
            .handle(GetProgressDailyDeltasQuery {
                item_id: ProgressItemId(1),
                date_range: DateRange::new(
                    NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
                    NaiveDate::from_ymd_opt(2026, 5, 4).unwrap(),
                )
                .unwrap(),
            })
            .unwrap();

        let values: Vec<_> = result
            .into_iter()
            .map(|dated| (dated.date, dated.value))
            .collect();
        assert_eq!(
            values,
            vec![
                (NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(), 40),
                (NaiveDate::from_ymd_opt(2026, 5, 3).unwrap(), 5),
                (NaiveDate::from_ymd_opt(2026, 5, 4).unwrap(), 15),
            ]
        );
    }

    #[test]
    fn daily_deltas_respect_frontier_before_requested_range() {
        let checkpoints = vec![checkpoint(1, 1, 1, 50), checkpoint(2, 2, 2, 60)];
        let mut repo = MockProgressRepository::new();
        repo.expect_find_item_by_id()
            .times(1)
            .returning(|_| Ok(Some(item())));
        repo.expect_find_checkpoints()
            .times(1)
            .returning(move |_, _| Ok(checkpoints.clone()));

        let handler = GetProgressDailyDeltasQueryHandler::new(Arc::new(repo));
        let result = handler
            .handle(GetProgressDailyDeltasQuery {
                item_id: ProgressItemId(1),
                date_range: DateRange::new(
                    NaiveDate::from_ymd_opt(2026, 5, 2).unwrap(),
                    NaiveDate::from_ymd_opt(2026, 5, 2).unwrap(),
                )
                .unwrap(),
            })
            .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].value, 10);
    }
}
