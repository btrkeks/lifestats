use std::sync::Arc;

use events::event_port::EventPublisher;
use lifestats_application::use_cases::{
    goal_management_service::GoalManagementService, logging_service::LoggingService,
    statistic_query_service::StatisticQueryService,
};
use lifestats_domain::statistic::ports::statistic_repository::StatisticRepository;

/// Shared application state injected into every request handler
pub struct AppState<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    pub logging_service: Arc<LoggingService<R, E>>,
    pub goal_service: Arc<GoalManagementService<R, E>>,
    pub query_service: Arc<StatisticQueryService<R>>,
}

impl<R, E> Clone for AppState<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    fn clone(&self) -> Self {
        Self {
            logging_service: Arc::clone(&self.logging_service),
            goal_service: Arc::clone(&self.goal_service),
            query_service: Arc::clone(&self.query_service),
        }
    }
}

impl<R, E> AppState<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    pub fn new(
        logging_service: Arc<LoggingService<R, E>>,
        goal_service: Arc<GoalManagementService<R, E>>,
        query_service: Arc<StatisticQueryService<R>>,
    ) -> Self {
        Self {
            logging_service,
            goal_service,
            query_service,
        }
    }
}
