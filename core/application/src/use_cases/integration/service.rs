use lifestats_core::ebook::{
    BookKey, BookTitle, EbookDayReport, EbookError, EbookReadingStore, EbookWriteOutcome,
    ProducerId,
};
use std::collections::HashMap;
use std::num::NonZeroU64;
use std::sync::Arc;

use super::bundle_factory::BundleFactory;
use super::ebook_event_store::{EbookEventStore, EbookSyncEvent};
use super::ebook_progress_service::{EbookProgressServiceTrait, EbookProgressSyncRequest};
use super::liftlog_bundle_factory::{LIFTLOG_SOURCE_TAG, METRIC_WORKOUT_TIME};
use super::liftlog_days::{LiftLogDaysSyncResult, LiftLogWorkout};
use crate::error::{ApplicationError, ApplicationResult};
use chrono::{NaiveDate, NaiveTime, Utc};
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::shared::time_frame_list::{TimeFrame, TimeFrameList};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use std::collections::BTreeMap;

#[cfg(test)]
use mockall::automock;

const EBOOK_SOURCE: &str = "ebook";
const EBOOK_METRIC_CHARACTERS_READ: &str = "characters_read";

/// Trait defining the contract for integration service implementations.
///
/// This trait allows for dependency injection and testability.
#[cfg_attr(test, automock)]
pub trait IntegrationServiceTrait: Send + Sync {
    fn ebook_server_id(&self) -> Result<String, EbookError>;
    fn record_ebook_day(
        &self,
        producer: ProducerId,
        book: BookKey,
        day: NaiveDate,
        report: EbookDayReport,
    ) -> Result<EbookWriteOutcome, EbookError>;
    fn bind_legacy_ebook_book(
        &self,
        reader_data_id: u32,
        book: BookKey,
        title: BookTitle,
        length: NonZeroU64,
    ) -> Result<EbookWriteOutcome, EbookError>;

    fn replace_yomikata_day(
        &self,
        day: NaiveDate,
        totals: lifestats_core::yomikata::YomikataDayTotals,
    ) -> Result<
        lifestats_core::yomikata::YomikataDayReplacement,
        lifestats_core::yomikata::YomikataDayError,
    >;

    /// Provisions a bundle of statistics for the given source and context.
    ///
    /// This method is idempotent: calling it multiple times with the same
    /// source and context will return the same statistic IDs.
    ///
    /// Returns a map from logical key (e.g., "time_spent") to StatisticId.
    fn provision_bundle(
        &self,
        source: &str,
        context: HashMap<String, String>,
    ) -> ApplicationResult<HashMap<String, StatisticId>>;

    /// Accepts an idempotent ebook sync event and applies it at most once.
    fn ingest_ebook_event(
        &self,
        request: EbookEventIngestionRequest,
    ) -> ApplicationResult<EbookEventIngestionResult>;

    /// Replaces `[LiftLog] Workout Time` on exactly the given days with the
    /// given workouts. A day with no workouts loses its value. Days not named
    /// are untouched, and resending the same days changes nothing.
    fn sync_liftlog_days(
        &self,
        days: BTreeMap<NaiveDate, Vec<LiftLogWorkout>>,
    ) -> ApplicationResult<LiftLogDaysSyncResult>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EbookEventIngestionRequest {
    pub event_id: String,
    pub date: NaiveDate,
    pub metric: String,
    pub delta: i64,
    pub book_id: String,
    pub book_title: String,
    pub confirmed_position_after: i64,
    pub book_target_characters: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EbookEventIngestionResult {
    pub accepted: bool,
    pub statistic_id: StatisticId,
}

/// Service for provisioning integration bundles in an idempotent manner.
///
/// This service ensures that multiple calls with the same source and context
/// return the same statistic IDs without creating duplicates.
pub struct IntegrationService {
    ebook_reading_store: Option<Arc<dyn EbookReadingStore>>,
    yomikata_day_store: Option<Arc<dyn lifestats_core::yomikata::YomikataDayStore>>,
    repository: Arc<dyn StatisticRepository + Send + Sync>,
    factories: HashMap<String, Arc<dyn BundleFactory>>,
    ebook_event_store: Option<Arc<dyn EbookEventStore>>,
    ebook_progress_service: Option<Arc<dyn EbookProgressServiceTrait>>,
}

impl IntegrationService {
    /// Creates a new IntegrationService with the given repository and factories.
    pub fn new(
        repository: Arc<dyn StatisticRepository + Send + Sync>,
        factories: Vec<Arc<dyn BundleFactory>>,
        ebook_event_store: Option<Arc<dyn EbookEventStore>>,
        ebook_progress_service: Option<Arc<dyn EbookProgressServiceTrait>>,
    ) -> Self {
        // Build a map from source_name to factory for O(1) lookup
        let factories_map = factories
            .into_iter()
            .map(|factory| (factory.source_name().to_string(), factory))
            .collect();

        Self {
            ebook_reading_store: None,
            yomikata_day_store: None,
            repository,
            factories: factories_map,
            ebook_event_store,
            ebook_progress_service,
        }
    }

    pub fn with_ebook_reading_store(mut self, store: Arc<dyn EbookReadingStore>) -> Self {
        self.ebook_reading_store = Some(store);
        self
    }
    fn ebook_store(&self) -> Result<&dyn EbookReadingStore, EbookError> {
        self.ebook_reading_store.as_deref().ok_or_else(|| {
            EbookError::Storage(
                lifestats_core::shared::ports::error::RepositoryError::OperationFailed(
                    anyhow::anyhow!("Ebook reading store is not configured"),
                ),
            )
        })
    }

    pub fn with_yomikata_day_store(
        mut self,
        store: Arc<dyn lifestats_core::yomikata::YomikataDayStore>,
    ) -> Self {
        self.yomikata_day_store = Some(store);
        self
    }

    /// Writes a log value only when it differs from the stored one.
    /// Returns whether the stored value changed.
    fn save_log_value_if_changed(
        &self,
        statistic_id: StatisticId,
        date: NaiveDate,
        value: serde_json::Value,
    ) -> ApplicationResult<bool> {
        let current = self
            .repository
            .find_log_value(statistic_id, date)
            .map_err(ApplicationError::Repository)?;

        if current.as_ref() == Some(&value) {
            return Ok(false);
        }

        self.repository
            .save_log_value(statistic_id, date, value)
            .map_err(ApplicationError::Repository)?;
        Ok(true)
    }
}

/// Builds one merged, non-overlapping time-frame list for a day's workouts.
///
/// Timestamps are clamped into the session's own calendar date (LiftLog
/// sessions can technically cross midnight), and touching or overlapping
/// intervals from multiple sessions on the same day are merged.
fn day_time_frames(date: NaiveDate, workouts: &[LiftLogWorkout]) -> TimeFrameList {
    use chrono::Timelike;

    const DAY_END_SECS: i64 = 23 * 3600 + 59 * 60 + 59;
    const MIN_FRAME_SECS: i64 = 60;

    let day_start = NaiveTime::from_hms_opt(0, 0, 0).expect("valid time");
    let day_end = NaiveTime::from_hms_opt(23, 59, 59).expect("valid time");
    let time_from_secs = |secs: i64| {
        NaiveTime::from_num_seconds_from_midnight_opt(secs as u32, 0).expect("clamped day seconds")
    };

    let mut intervals: Vec<(NaiveTime, NaiveTime)> = Vec::new();
    for workout in workouts {
        // Clamp both endpoints into the attributed calendar date.
        let start = match workout.started_at.date().cmp(&date) {
            std::cmp::Ordering::Less => day_start,
            std::cmp::Ordering::Equal => workout.started_at.time(),
            std::cmp::Ordering::Greater => continue,
        };
        let end = match workout.finished_at.date().cmp(&date) {
            std::cmp::Ordering::Less => continue,
            std::cmp::Ordering::Equal => workout.finished_at.time(),
            std::cmp::Ordering::Greater => day_end,
        };

        // Timestamps mark set completions, so a single-set session collapses
        // to an instant; give every workout a visible minimum length.
        let start_secs = i64::from(start.num_seconds_from_midnight());
        let mut end_secs = i64::from(end.num_seconds_from_midnight()).max(start_secs);
        let mut start_secs = start_secs;
        if end_secs - start_secs < MIN_FRAME_SECS {
            end_secs = (start_secs + MIN_FRAME_SECS).min(DAY_END_SECS);
            start_secs = (end_secs - MIN_FRAME_SECS).max(0);
        }

        intervals.push((time_from_secs(start_secs), time_from_secs(end_secs)));
    }

    intervals.sort();
    let mut merged: Vec<(NaiveTime, NaiveTime)> = Vec::new();
    for (start, end) in intervals {
        match merged.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }

    let mut frames = TimeFrameList::default();
    for (start, end) in merged {
        if let Ok(frame) = TimeFrame::new(start, end) {
            // Merged intervals are sorted and disjoint, so this cannot fail.
            frames
                .append_frame(frame)
                .expect("merged intervals must not overlap");
        }
    }
    frames
}

impl IntegrationServiceTrait for IntegrationService {
    fn ebook_server_id(&self) -> Result<String, EbookError> {
        self.ebook_store()?.server_id()
    }
    fn record_ebook_day(
        &self,
        producer: ProducerId,
        book: BookKey,
        day: NaiveDate,
        report: EbookDayReport,
    ) -> Result<EbookWriteOutcome, EbookError> {
        self.ebook_store()?.record_day(
            producer,
            book,
            day,
            report,
            super::ebook_bundle_factory::EbookBundleFactory::definitions(),
        )
    }
    fn bind_legacy_ebook_book(
        &self,
        reader_data_id: u32,
        book: BookKey,
        title: BookTitle,
        length: NonZeroU64,
    ) -> Result<EbookWriteOutcome, EbookError> {
        self.ebook_store()?.bind_legacy_book(
            reader_data_id,
            book,
            title,
            length,
            super::ebook_bundle_factory::EbookBundleFactory::definitions(),
        )
    }

    fn replace_yomikata_day(
        &self,
        day: NaiveDate,
        totals: lifestats_core::yomikata::YomikataDayTotals,
    ) -> Result<
        lifestats_core::yomikata::YomikataDayReplacement,
        lifestats_core::yomikata::YomikataDayError,
    > {
        let store = self.yomikata_day_store.as_ref().ok_or_else(|| {
            lifestats_core::yomikata::YomikataDayError::Storage(
                lifestats_core::shared::ports::error::RepositoryError::OperationFailed(
                    anyhow::anyhow!("Yomikata day store is not configured"),
                ),
            )
        })?;
        store.replace_day(
            day,
            totals,
            super::yomikata_bundle_factory::YomikataBundleFactory::definitions(),
        )
    }

    /// Provisions a bundle of statistics for the given source and context.
    ///
    /// This method is idempotent: calling it multiple times with the same
    /// source and context will return the same statistic IDs.
    ///
    /// Returns a map from logical key (e.g., "time_spent") to StatisticId.
    fn provision_bundle(
        &self,
        source: &str,
        context: HashMap<String, String>,
    ) -> ApplicationResult<HashMap<String, StatisticId>> {
        let factory = self
            .factories
            .get(source)
            .ok_or_else(|| ApplicationError::InvalidInput(format!("Unknown source: {}", source)))?;

        let provisioned_stats = factory.provision(&context)?;

        let mut result = HashMap::new();
        let mut statistics_to_create = Vec::new();
        let mut keys_to_create = Vec::new();

        // Each provisioned statistic defines canonical tags. Reuse them to look up
        // existing metadata so factories control idempotency via their tag design.
        for provisioned in provisioned_stats {
            let tags = provisioned.statistic.get_metadata().tags().clone();

            let existing = self
                .repository
                .find_by_tags(&tags)
                .map_err(ApplicationError::Repository)?;

            if let Some(statistic) = existing.into_iter().next() {
                result.insert(provisioned.key, statistic.get_metadata().id());
            } else {
                keys_to_create.push(provisioned.key);
                statistics_to_create.push(provisioned.statistic);
            }
        }

        if statistics_to_create.is_empty() {
            return Ok(result);
        }

        let persisted = self
            .repository
            .create_many_metadata(statistics_to_create)
            .map_err(ApplicationError::Repository)?;

        for (key, statistic) in keys_to_create.into_iter().zip(persisted.into_iter()) {
            result.insert(key, statistic.get_metadata().id());
        }

        Ok(result)
    }

    fn ingest_ebook_event(
        &self,
        request: EbookEventIngestionRequest,
    ) -> ApplicationResult<EbookEventIngestionResult> {
        if request.event_id.trim().is_empty() {
            return Err(ApplicationError::InvalidInput(
                "ebook event_id must not be empty".to_string(),
            ));
        }

        if request.metric != EBOOK_METRIC_CHARACTERS_READ {
            return Err(ApplicationError::InvalidInput(format!(
                "unsupported ebook metric: {}",
                request.metric
            )));
        }

        if request.delta <= 0 {
            return Err(ApplicationError::InvalidInput(
                "ebook delta must be positive".to_string(),
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

        let ebook_event_store = self.ebook_event_store.as_ref().ok_or_else(|| {
            ApplicationError::InvalidInput(
                "ebook event ingestion is not configured for this deployment".to_string(),
            )
        })?;
        let ebook_progress_service = self.ebook_progress_service.as_ref().ok_or_else(|| {
            ApplicationError::InvalidInput(
                "ebook progress syncing is not configured for this deployment".to_string(),
            )
        })?;

        let statistic_ids = self.provision_bundle(EBOOK_SOURCE, HashMap::new())?;
        let statistic_id = statistic_ids
            .get(EBOOK_METRIC_CHARACTERS_READ)
            .copied()
            .ok_or_else(|| {
                ApplicationError::Infrastructure(anyhow::anyhow!(
                    "ebook bundle did not provision the characters_read statistic"
                ))
            })?;

        let event = EbookSyncEvent {
            event_id: request.event_id.clone(),
            source_name: EBOOK_SOURCE.to_string(),
            metric: request.metric.clone(),
            statistic_id,
            date: request.date,
            delta: request.delta,
            book_id: Some(request.book_id.clone()),
            book_title: request.book_title.clone(),
            confirmed_position_after: request.confirmed_position_after,
        };

        let accepted = ebook_event_store.record_event(&event)?;

        ebook_progress_service.ensure_and_log_checkpoint(EbookProgressSyncRequest {
            event_id: request.event_id,
            book_id: request.book_id,
            book_title: request.book_title,
            effective_date: request.date,
            recorded_at: Utc::now(),
            confirmed_position_after: request.confirmed_position_after,
            book_target_characters: request.book_target_characters,
        })?;

        Ok(EbookEventIngestionResult {
            accepted,
            statistic_id,
        })
    }

    fn sync_liftlog_days(
        &self,
        days: BTreeMap<NaiveDate, Vec<LiftLogWorkout>>,
    ) -> ApplicationResult<LiftLogDaysSyncResult> {
        let statistic_ids = self.provision_bundle(LIFTLOG_SOURCE_TAG, HashMap::new())?;
        let statistic_id = *statistic_ids.get(METRIC_WORKOUT_TIME).ok_or_else(|| {
            ApplicationError::Infrastructure(anyhow::anyhow!(
                "liftlog bundle did not provision the workout_time statistic"
            ))
        })?;

        let mut result = LiftLogDaysSyncResult {
            statistic_id,
            days_updated: 0,
            days_cleared: 0,
            days_unchanged: 0,
        };
        for (date, workouts) in days {
            let frames = day_time_frames(date, &workouts);
            if frames.is_empty() {
                let deleted = self
                    .repository
                    .delete_log_value(statistic_id, date)
                    .map_err(ApplicationError::Repository)?;
                if deleted {
                    result.days_cleared += 1;
                } else {
                    result.days_unchanged += 1;
                }
                continue;
            }
            let value = serde_json::to_value(&frames).map_err(|e| {
                ApplicationError::Infrastructure(anyhow::anyhow!(
                    "failed to serialize time frames: {e}"
                ))
            })?;
            if self.save_log_value_if_changed(statistic_id, date, value)? {
                result.days_updated += 1;
            } else {
                result.days_unchanged += 1;
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::use_cases::integration::liftlog_bundle_factory::LiftLogBundleFactory;
    use chrono::NaiveDateTime;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;

    const WORKOUT_TIME_ID: StatisticId = StatisticId(10);
    const TOTAL_WEIGHT_ID: StatisticId = StatisticId(11);

    fn local(date: (i32, u32, u32), time: (u32, u32, u32)) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(date.0, date.1, date.2)
            .unwrap()
            .and_hms_opt(time.0, time.1, time.2)
            .unwrap()
    }

    /// Makes provisioning resolve to fixed statistic IDs by returning an
    /// existing statistic for every tag lookup.
    fn expect_provisioned_bundle(repository: &mut MockStatisticRepository) {
        use lifestats_core::statistic::any_statistic::AnyStatistic;
        use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
        use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

        repository.expect_find_by_tags().returning(|tags| {
            let id = if tags.get("metric").map(String::as_str) == Some(METRIC_WORKOUT_TIME) {
                WORKOUT_TIME_ID
            } else {
                TOTAL_WEIGHT_ID
            };
            let metadata =
                StatisticMetadata::new(id, "LiftLog".to_string(), format!("stat {}", id.0));
            let statistic = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
                metadata,
                None,
                NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            ));
            Ok(vec![statistic])
        });
    }

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 8, d).unwrap()
    }

    fn span(d: u32, start: (u32, u32, u32), end: (u32, u32, u32)) -> LiftLogWorkout {
        LiftLogWorkout {
            started_at: local((2026, 8, d), start),
            finished_at: local((2026, 8, d), end),
        }
    }

    fn sync_service(repository: MockStatisticRepository) -> IntegrationService {
        IntegrationService::new(
            Arc::new(repository),
            vec![Arc::new(LiftLogBundleFactory)],
            None,
            None,
        )
    }

    #[test]
    fn sync_liftlog_days_writes_merged_frames_for_a_day() {
        let mut repository = MockStatisticRepository::new();
        expect_provisioned_bundle(&mut repository);
        repository
            .expect_find_log_value()
            .returning(|_, _| Ok(None));
        repository
            .expect_save_log_value()
            .withf(|id, date, value| {
                *id == WORKOUT_TIME_ID
                    && *date == day(12)
                    && value == &serde_json::json!([{"start": "18:00:00", "end": "18:55:00"}])
            })
            .times(1)
            .returning(|_, _, _| Ok(()));

        let days = BTreeMap::from([(
            day(12),
            vec![
                span(12, (18, 0, 0), (18, 30, 0)),
                span(12, (18, 20, 0), (18, 55, 0)),
            ],
        )]);
        let result = sync_service(repository).sync_liftlog_days(days).unwrap();

        assert_eq!(
            result,
            LiftLogDaysSyncResult {
                statistic_id: WORKOUT_TIME_ID,
                days_updated: 1,
                days_cleared: 0,
                days_unchanged: 0,
            }
        );
    }

    #[test]
    fn sync_liftlog_days_clears_empty_days_and_counts_absent_ones_as_unchanged() {
        let mut repository = MockStatisticRepository::new();
        expect_provisioned_bundle(&mut repository);
        repository
            .expect_delete_log_value()
            .withf(|id, date| *id == WORKOUT_TIME_ID && *date == day(9))
            .times(1)
            .returning(|_, _| Ok(true));
        repository
            .expect_delete_log_value()
            .withf(|id, date| *id == WORKOUT_TIME_ID && *date == day(10))
            .times(1)
            .returning(|_, _| Ok(false));
        repository.expect_save_log_value().times(0);

        let days = BTreeMap::from([(day(9), vec![]), (day(10), vec![])]);
        let result = sync_service(repository).sync_liftlog_days(days).unwrap();

        assert_eq!(
            result,
            LiftLogDaysSyncResult {
                statistic_id: WORKOUT_TIME_ID,
                days_updated: 0,
                days_cleared: 1,
                days_unchanged: 1,
            }
        );
        assert!(result.changed());
    }

    #[test]
    fn sync_liftlog_days_leaves_matching_values_alone() {
        let mut repository = MockStatisticRepository::new();
        expect_provisioned_bundle(&mut repository);
        repository
            .expect_find_log_value()
            .withf(|id, date| *id == WORKOUT_TIME_ID && *date == day(12))
            .returning(|_, _| {
                Ok(Some(serde_json::json!([
                    {"start": "18:00:00", "end": "18:30:00"}
                ])))
            });
        repository.expect_save_log_value().times(0);

        let days = BTreeMap::from([(day(12), vec![span(12, (18, 0, 0), (18, 30, 0))])]);
        let result = sync_service(repository).sync_liftlog_days(days).unwrap();

        assert_eq!(
            result,
            LiftLogDaysSyncResult {
                statistic_id: WORKOUT_TIME_ID,
                days_updated: 0,
                days_cleared: 0,
                days_unchanged: 1,
            }
        );
        assert!(!result.changed());
    }
}
