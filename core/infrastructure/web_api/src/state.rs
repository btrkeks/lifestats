use std::sync::Arc;

use events::event_port::EventPublisher;
use lifestats_application::use_cases::{
    goal_management_service::GoalManagementService, integration::IntegrationServiceTrait,
    logging_service::LoggingService, plotting_service::PlottingService,
    statistic_query_service::StatisticQueryService,
};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use plot_domain::plot::renderer::PlotRenderer;
use plot_domain::plot::repository::PlotRepository;

/// Shared application state injected into every request handler
pub struct AppState<R, P, Rend, E, I>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    pub logging_service: Arc<LoggingService<R, E>>,
    pub goal_service: Arc<GoalManagementService<R, E>>,
    pub query_service: Arc<StatisticQueryService<R>>,
    pub plotting_service: Arc<PlottingService<R, P, Rend>>,
    pub integration_service: Arc<I>,
}

impl<R, P, Rend, E, I> Clone for AppState<R, P, Rend, E, I>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    fn clone(&self) -> Self {
        Self {
            logging_service: Arc::clone(&self.logging_service),
            goal_service: Arc::clone(&self.goal_service),
            query_service: Arc::clone(&self.query_service),
            plotting_service: Arc::clone(&self.plotting_service),
            integration_service: Arc::clone(&self.integration_service),
        }
    }
}

impl<R, P, Rend, E, I> AppState<R, P, Rend, E, I>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    pub fn new(
        logging_service: Arc<LoggingService<R, E>>,
        goal_service: Arc<GoalManagementService<R, E>>,
        query_service: Arc<StatisticQueryService<R>>,
        plotting_service: Arc<PlottingService<R, P, Rend>>,
        integration_service: Arc<I>,
    ) -> Self {
        Self {
            logging_service,
            goal_service,
            query_service,
            plotting_service,
            integration_service,
        }
    }
}
