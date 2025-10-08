use std::sync::Arc;

use events::event_port::EventPublisher;
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use plot_domain::plot::renderer::PlotRenderer;
use plot_domain::plot::repository::PlotRepository;

/// Shared application state injected into every request handler
///
/// This contains only globally shared dependencies. Feature-specific
/// dependencies (like Fitbit integration) are injected via Axum layers.
pub struct AppState<R, P, Rend, E, I>
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    // Direct repository access for CQRS handlers
    pub statistic_repository: Arc<R>,
    pub plot_repository: Arc<P>,
    pub event_publisher: Arc<E>,
    pub renderer: Arc<Rend>,
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
            statistic_repository: Arc::clone(&self.statistic_repository),
            plot_repository: Arc::clone(&self.plot_repository),
            event_publisher: Arc::clone(&self.event_publisher),
            renderer: Arc::clone(&self.renderer),
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
        statistic_repository: Arc<R>,
        plot_repository: Arc<P>,
        event_publisher: Arc<E>,
        renderer: Arc<Rend>,
        integration_service: Arc<I>,
    ) -> Self {
        Self {
            statistic_repository,
            plot_repository,
            event_publisher,
            renderer,
            integration_service,
        }
    }
}
