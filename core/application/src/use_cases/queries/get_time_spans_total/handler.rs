use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use anyhow::anyhow;
use lifestats_core::shared::time_frame_list::TimeFrameList;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use super::query::{GetTimeSpansTotalQuery, TimeSpansTotalResult};

pub struct GetTimeSpansTotalQueryHandler<S>
where
    S: StatisticRepository + Send + Sync + 'static,
{
    repository: Arc<S>,
}

impl<S> GetTimeSpansTotalQueryHandler<S>
where
    S: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(repository: Arc<S>) -> Self {
        Self { repository }
    }
}

impl<S> QueryHandler<GetTimeSpansTotalQuery> for GetTimeSpansTotalQueryHandler<S>
where
    S: StatisticRepository + Send + Sync + 'static,
{
    fn handle(&self, query: GetTimeSpansTotalQuery) -> ApplicationResult<TimeSpansTotalResult> {
        // 1. Verify statistic exists and is a TimeSpans type
        let statistic = self.repository.find_metadata_by_id(query.statistic_id)?;

        if !matches!(statistic, AnyStatistic::TimeSpans(_)) {
            return Err(anyhow!("Statistic {} is not a TimeSpans type", query.statistic_id).into());
        }

        // 2. Fetch all log values in the date range
        let log_values = self
            .repository
            .find_log_values_in_range(query.statistic_id, query.date_range)?;

        // 3. Deserialize each JSON value to TimeFrameList and sum durations
        let mut total_seconds: i64 = 0;
        let days_with_data = log_values.len();

        for dated_value in log_values {
            let time_frame_list = serde_json::from_value::<TimeFrameList>(dated_value.value)
                .map_err(|e| anyhow!("Failed to parse TimeFrameList: {}", e))?;

            total_seconds += time_frame_list.get_total_duration().num_seconds();
        }

        // 4. Return result
        Ok(TimeSpansTotalResult {
            statistic_id: query.statistic_id,
            start_date: query.date_range.start(),
            end_date: query.date_range.end(),
            total_seconds,
            days_with_data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_core::shared::date_range::DateRange;
    use lifestats_core::shared::dated_value::DatedValue;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use serde_json::json;

    #[test]
    fn test_get_time_spans_total() {
        let mut repo = MockStatisticRepository::new();
        let stat_id = StatisticId::from(1);

        let statistic = AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            StatisticMetadata::new(stat_id, "Test".into(), "Study Time".into()),
            None,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        ));

        repo.expect_find_metadata_by_id()
            .returning(move |_| Ok(statistic.clone()));

        // 2 days with data:
        // Day 1: 10:00-12:00 = 2 hours = 7200 seconds
        // Day 2: 14:00-15:30 = 1.5 hours = 5400 seconds
        // Total: 12600 seconds
        repo.expect_find_log_values_in_range().returning(|_, _| {
            Ok(vec![
                DatedValue::new(
                    NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(),
                    json!([{"start": "10:00:00", "end": "12:00:00"}]),
                ),
                DatedValue::new(
                    NaiveDate::from_ymd_opt(2025, 10, 2).unwrap(),
                    json!([{"start": "14:00:00", "end": "15:30:00"}]),
                ),
            ])
        });

        let handler = GetTimeSpansTotalQueryHandler::new(Arc::new(repo));
        let query = GetTimeSpansTotalQuery {
            statistic_id: stat_id,
            date_range: DateRange::new(
                NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 10, 10).unwrap(),
            )
            .unwrap(),
        };

        let result = handler.handle(query).unwrap();

        assert_eq!(result.total_seconds, 12600);
        assert_eq!(result.days_with_data, 2);
    }

    #[test]
    fn test_rejects_non_time_spans_statistic() {
        let mut repo = MockStatisticRepository::new();
        let stat_id = StatisticId::from(1);

        // Return a Habit statistic instead of TimeSpans
        use lifestats_core::statistic::types::logable_statistics::habit::HabitStatistic;
        let statistic = AnyStatistic::Habit(HabitStatistic::new(
            StatisticMetadata::new(stat_id, "Test".into(), "Daily Exercise".into()),
            None,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        ));

        repo.expect_find_metadata_by_id()
            .returning(move |_| Ok(statistic.clone()));

        let handler = GetTimeSpansTotalQueryHandler::new(Arc::new(repo));
        let query = GetTimeSpansTotalQuery {
            statistic_id: stat_id,
            date_range: DateRange::new(
                NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 10, 10).unwrap(),
            )
            .unwrap(),
        };

        let result = handler.handle(query);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not a TimeSpans type"));
    }

    #[test]
    fn test_empty_date_range() {
        let mut repo = MockStatisticRepository::new();
        let stat_id = StatisticId::from(1);

        let statistic = AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            StatisticMetadata::new(stat_id, "Test".into(), "Study Time".into()),
            None,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        ));

        repo.expect_find_metadata_by_id()
            .returning(move |_| Ok(statistic.clone()));

        repo.expect_find_log_values_in_range()
            .returning(|_, _| Ok(vec![]));

        let handler = GetTimeSpansTotalQueryHandler::new(Arc::new(repo));
        let query = GetTimeSpansTotalQuery {
            statistic_id: stat_id,
            date_range: DateRange::new(
                NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 10, 10).unwrap(),
            )
            .unwrap(),
        };

        let result = handler.handle(query).unwrap();

        assert_eq!(result.total_seconds, 0);
        assert_eq!(result.days_with_data, 0);
    }

    #[test]
    fn test_multiple_time_frames_per_day() {
        let mut repo = MockStatisticRepository::new();
        let stat_id = StatisticId::from(1);

        let statistic = AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            StatisticMetadata::new(stat_id, "Test".into(), "Study Time".into()),
            None,
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        ));

        repo.expect_find_metadata_by_id()
            .returning(move |_| Ok(statistic.clone()));

        // Single day with multiple time frames:
        // 10:00-11:00 = 1 hour = 3600 seconds
        // 14:00-15:30 = 1.5 hours = 5400 seconds
        // Total: 9000 seconds
        repo.expect_find_log_values_in_range().returning(|_, _| {
            Ok(vec![DatedValue::new(
                NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(),
                json!([
                    {"start": "10:00:00", "end": "11:00:00"},
                    {"start": "14:00:00", "end": "15:30:00"}
                ]),
            )])
        });

        let handler = GetTimeSpansTotalQueryHandler::new(Arc::new(repo));
        let query = GetTimeSpansTotalQuery {
            statistic_id: stat_id,
            date_range: DateRange::new(
                NaiveDate::from_ymd_opt(2025, 10, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 10, 10).unwrap(),
            )
            .unwrap(),
        };

        let result = handler.handle(query).unwrap();

        assert_eq!(result.total_seconds, 9000);
        assert_eq!(result.days_with_data, 1);
    }
}
