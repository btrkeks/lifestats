use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use crate::use_cases::queries::{GetAllTimelineEventsQuery, GetAllTimelineEventsQueryHandler};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use super::query::GetDailyTimelinePlotQuery;

/// Handler for GetDailyTimelinePlotQuery.
///
/// This handler orchestrates fetching timeline events and rendering them as a plot.
pub struct GetDailyTimelinePlotQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    repository: Arc<R>,
}

impl<R> GetDailyTimelinePlotQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R> QueryHandler<GetDailyTimelinePlotQuery> for GetDailyTimelinePlotQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    fn handle(&self, query: GetDailyTimelinePlotQuery) -> ApplicationResult<Vec<u8>> {
        // 1. Fetch all timeline events for the date
        let timeline_events = GetAllTimelineEventsQueryHandler::new(self.repository.clone())
            .handle(GetAllTimelineEventsQuery { date: query.date })?;

        // 2. Convert TimelineEvent to TimelineEventData (to avoid circular dependency)
        let event_data: Vec<lifestats_plotting::TimelineEventData> = timeline_events
            .into_iter()
            .map(|event| lifestats_plotting::TimelineEventData {
                timeframe: event.timeframe,
                label: event.label,
            })
            .collect();

        // 3. Render the timeline plot
        let plot_bytes = lifestats_plotting::plot_timeline_events(
            event_data,
            query.date,
            plot_domain::plot::renderer::RenderConfig::new(800, 1500),
        )?;

        Ok(plot_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::any_statistic::AnyStatistic;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use serde_json::json;

    #[test]
    fn test_returns_plot_bytes() {
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

        repo.expect_find_log_value().returning(|_, _| {
            Ok(Some(json!([
                {"start": "10:00:00", "end": "12:00:00"}
            ])))
        });

        let handler = GetDailyTimelinePlotQueryHandler::new(Arc::new(repo));
        let query = GetDailyTimelinePlotQuery { date: query_date };

        // Act
        let result = handler.handle(query);

        // Assert
        assert!(result.is_ok(), "Handler should successfully generate plot");
        let png_bytes = result.unwrap();
        assert!(!png_bytes.is_empty(), "PNG data should not be empty");

        // Verify it's a valid PNG by checking magic bytes
        assert_eq!(
            &png_bytes[0..4],
            &[0x89, 0x50, 0x4E, 0x47],
            "Should be a valid PNG"
        );
    }
}
