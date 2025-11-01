use std::collections::HashMap;
use std::sync::Arc;

use crate::cqrs::{CommandHandler, QueryHandler};
use crate::dto::summary::TimelineEvent;
use crate::dto::DailySummaryResponse;
use crate::error::ApplicationResult;
use crate::use_cases::commands::{CalculatePercentagesCommand, CalculatePercentagesCommandHandler};
use crate::use_cases::queries::{GetAllTimelineEventsQuery, GetAllTimelineEventsQueryHandler};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use super::query::GetDailySummaryQuery;

/// Handler for GetDailySummaryQuery.
pub struct GetDailySummaryQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    repository: Arc<R>,
}

impl<R> GetDailySummaryQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }
}

impl<R> QueryHandler<GetDailySummaryQuery> for GetDailySummaryQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    fn handle(&self, query: GetDailySummaryQuery) -> ApplicationResult<DailySummaryResponse> {
        let all_timeline_events: Vec<TimelineEvent> =
            GetAllTimelineEventsQueryHandler::new(self.repository.clone())
                .handle(GetAllTimelineEventsQuery { date: query.date })?;

        let category_percentages: HashMap<String, f64> = CalculatePercentagesCommandHandler
            .handle(CalculatePercentagesCommand {
                timeline_events: all_timeline_events.clone(),
            })?;

        Ok(DailySummaryResponse {
            date: query.date,
            category_percentages,
            timeline_events: all_timeline_events,
        })
    }
}
