use std::sync::Arc;

use events::event_port::EventPublisher;
use lifestats_application::use_cases::{
    goal_management_service::GoalManagementService, logging_service::LoggingService,
    plotting_service::PlottingService, statistic_query_service::StatisticQueryService,
};
use lifestats_domain::plot::renderer::PlotRenderer;
use lifestats_domain::plot::repository::PlotRepository;
use lifestats_domain::statistic::ports::statistic_repository::StatisticRepository;

/// Shared application state injected into every request handler
pub struct AppState<R, P, Rend, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
{
    pub logging_service: Arc<LoggingService<R, E>>,
    pub goal_service: Arc<GoalManagementService<R, E>>,
    pub query_service: Arc<StatisticQueryService<R>>,
    pub plotting_service: Arc<PlottingService<R, P, Rend>>,
}

impl<R, P, Rend, E> Clone for AppState<R, P, Rend, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
{
    fn clone(&self) -> Self {
        Self {
            logging_service: Arc::clone(&self.logging_service),
            goal_service: Arc::clone(&self.goal_service),
            query_service: Arc::clone(&self.query_service),
            plotting_service: Arc::clone(&self.plotting_service),
        }
    }
}

impl<R, P, Rend, E> AppState<R, P, Rend, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
{
    pub fn new(
        logging_service: Arc<LoggingService<R, E>>,
        goal_service: Arc<GoalManagementService<R, E>>,
        query_service: Arc<StatisticQueryService<R>>,
        plotting_service: Arc<PlottingService<R, P, Rend>>,
    ) -> Self {
        Self {
            logging_service,
            goal_service,
            query_service,
            plotting_service,
        }
    }
}
