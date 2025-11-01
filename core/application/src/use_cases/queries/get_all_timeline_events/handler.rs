use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::dto::summary::TimelineEvent;
use crate::error::ApplicationResult;
use crate::use_cases::queries::{GetTimeSpansMetadataQuery, GetTimeSpansMetadataQueryHandler};
use anyhow::anyhow;
use lifestats_core::shared::time_frame_list::TimeFrameList;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use super::query::GetAllTimelineEventsQuery;

/// Handler for GetAllTimelineEventsQuery.
pub struct GetAllTimelineEventsQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    repository: Arc<R>,
}

impl<R> GetAllTimelineEventsQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R> QueryHandler<GetAllTimelineEventsQuery> for GetAllTimelineEventsQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    fn handle(&self, query: GetAllTimelineEventsQuery) -> ApplicationResult<Vec<TimelineEvent>> {
        // 1. Get all TimeSpans statistics metadata
        let time_spans_metadata = GetTimeSpansMetadataQueryHandler::new(self.repository.clone())
            .handle(GetTimeSpansMetadataQuery)?;

        // 2. Collect timeline events from all TimeSpans statistics
        let mut timeline_events = Vec::new();

        for metadata in time_spans_metadata {
            // Fetch log value for the query date
            if let Some(log_value) = self.repository.find_log_value(metadata.id(), query.date)? {
                // Deserialize to TimeFrameList
                let time_frame_list: TimeFrameList = serde_json::from_value(log_value)
                    .map_err(|e| anyhow!("Failed to parse TimeFrameList: {}", e))?;

                // Create a TimelineEvent for each TimeFrame
                for time_frame in time_frame_list.iter() {
                    timeline_events.push(TimelineEvent {
                        timeframe: *time_frame,
                        label: metadata.name().to_string(),
                    });
                }
            }
        }

        // 3. Sort chronologically by start time
        timeline_events.sort_by_key(|event| event.timeframe.get_start());

        Ok(timeline_events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::any_statistic::AnyStatistic;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::habit::HabitStatistic;
    use lifestats_core::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use serde_json::json;

    #[test]
    fn test_single_statistic_with_multiple_timeframes() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let stat_id = StatisticId(1);
        let query_date = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();

        let statistic = AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            StatisticMetadata::new(stat_id, "Time".into(), "Study Time".into()),
            None,
            query_date,
        ));

        repo.expect_find_all_metadata()
            .returning(move || Ok(vec![statistic.clone()]));

        repo.expect_find_log_value()
            .withf(move |id, date| *id == stat_id && *date == query_date)
            .returning(|_, _| {
                Ok(Some(json!([
                    {"start": "10:00:00", "end": "12:00:00"},
                    {"start": "14:00:00", "end": "15:00:00"}
                ])))
            });

        let handler = GetAllTimelineEventsQueryHandler::new(Arc::new(repo));
        let query = GetAllTimelineEventsQuery { date: query_date };

        // Act
        let result = handler.handle(query);

        // Assert
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].label, "Study Time");
        assert_eq!(events[1].label, "Study Time");
    }

    #[test]
    fn test_multiple_statistics_with_chronological_sorting() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let study_id = StatisticId(1);
        let exercise_id = StatisticId(2);
        let query_date = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();

        let study_stat = AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            StatisticMetadata::new(study_id, "Time".into(), "Study Time".into()),
            None,
            query_date,
        ));

        let exercise_stat = AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            StatisticMetadata::new(exercise_id, "Time".into(), "Exercise".into()),
            None,
            query_date,
        ));

        repo.expect_find_all_metadata()
            .returning(move || Ok(vec![study_stat.clone(), exercise_stat.clone()]));

        repo.expect_find_log_value()
            .withf(move |id, date| *id == study_id && *date == query_date)
            .returning(|_, _| {
                Ok(Some(json!([
                    {"start": "10:00:00", "end": "12:00:00"},
                    {"start": "14:00:00", "end": "15:00:00"}
                ])))
            });

        repo.expect_find_log_value()
            .withf(move |id, date| *id == exercise_id && *date == query_date)
            .returning(|_, _| {
                Ok(Some(json!([
                    {"start": "09:00:00", "end": "09:30:00"}
                ])))
            });

        let handler = GetAllTimelineEventsQueryHandler::new(Arc::new(repo));
        let query = GetAllTimelineEventsQuery { date: query_date };

        // Act
        let result = handler.handle(query);

        // Assert
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 3);

        // Verify chronological order
        assert_eq!(events[0].label, "Exercise");
        assert_eq!(
            events[0]
                .timeframe
                .get_start()
                .format("%H:%M:%S")
                .to_string(),
            "09:00:00"
        );

        assert_eq!(events[1].label, "Study Time");
        assert_eq!(
            events[1]
                .timeframe
                .get_start()
                .format("%H:%M:%S")
                .to_string(),
            "10:00:00"
        );

        assert_eq!(events[2].label, "Study Time");
        assert_eq!(
            events[2]
                .timeframe
                .get_start()
                .format("%H:%M:%S")
                .to_string(),
            "14:00:00"
        );
    }

    #[test]
    fn test_no_data_for_query_date() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let stat_id = StatisticId(1);
        let query_date = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();

        let statistic = AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            StatisticMetadata::new(stat_id, "Time".into(), "Study Time".into()),
            None,
            query_date,
        ));

        repo.expect_find_all_metadata()
            .returning(move || Ok(vec![statistic.clone()]));

        repo.expect_find_log_value().returning(|_, _| Ok(None));

        let handler = GetAllTimelineEventsQueryHandler::new(Arc::new(repo));
        let query = GetAllTimelineEventsQuery { date: query_date };

        // Act
        let result = handler.handle(query);

        // Assert
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_no_time_spans_statistics_exist() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let query_date = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();

        // Return a Habit statistic (not TimeSpans)
        let habit_stat = AnyStatistic::Habit(HabitStatistic::new(
            StatisticMetadata::new(StatisticId(1), "Habit".into(), "Daily Exercise".into()),
            None,
            query_date,
        ));

        repo.expect_find_all_metadata()
            .returning(move || Ok(vec![habit_stat.clone()]));

        let handler = GetAllTimelineEventsQueryHandler::new(Arc::new(repo));
        let query = GetAllTimelineEventsQuery { date: query_date };

        // Act
        let result = handler.handle(query);

        // Assert
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 0);
    }

    #[test]
    fn test_mixed_statistics_only_time_spans_included() {
        // Arrange
        let mut repo = MockStatisticRepository::new();
        let time_spans_id = StatisticId(1);
        let habit_id = StatisticId(2);
        let query_date = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();

        let time_spans_stat = AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            StatisticMetadata::new(time_spans_id, "Time".into(), "Study Time".into()),
            None,
            query_date,
        ));

        let habit_stat = AnyStatistic::Habit(HabitStatistic::new(
            StatisticMetadata::new(habit_id, "Habit".into(), "Daily Exercise".into()),
            None,
            query_date,
        ));

        repo.expect_find_all_metadata()
            .returning(move || Ok(vec![time_spans_stat.clone(), habit_stat.clone()]));

        repo.expect_find_log_value()
            .withf(move |id, date| *id == time_spans_id && *date == query_date)
            .returning(|_, _| {
                Ok(Some(json!([
                    {"start": "10:00:00", "end": "12:00:00"}
                ])))
            });

        let handler = GetAllTimelineEventsQueryHandler::new(Arc::new(repo));
        let query = GetAllTimelineEventsQuery { date: query_date };

        // Act
        let result = handler.handle(query);

        // Assert
        assert!(result.is_ok());
        let events = result.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].label, "Study Time");
    }
}
