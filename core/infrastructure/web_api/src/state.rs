use std::sync::Arc;

use events::event_port::EventPublisher;
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_application::SessionManager;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use plot_domain::plot::renderer::PlotRenderer;

/// Shared application state injected into every request handler
///
/// This contains only globally shared dependencies. Feature-specific
/// dependencies (like Fitbit integration) are injected via Axum layers.
///
/// # Stateless Plotting
/// This state no longer includes PlotRepository - plotting is now stateless
/// and plots are generated on-the-fly using only the PlotRenderer.
pub struct AppState<R, Rend, E, I>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    // Direct repository access for CQRS handlers
    pub statistic_repository: Arc<R>,
    pub event_publisher: Arc<E>,
    pub renderer: Arc<Rend>,
    pub integration_service: Arc<I>,
    // Session management
    pub session_manager: Arc<SessionManager<R>>,
}

impl<R, Rend, E, I> Clone for AppState<R, Rend, E, I>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    fn clone(&self) -> Self {
        Self {
            statistic_repository: Arc::clone(&self.statistic_repository),
            event_publisher: Arc::clone(&self.event_publisher),
            renderer: Arc::clone(&self.renderer),
            integration_service: Arc::clone(&self.integration_service),
            session_manager: Arc::clone(&self.session_manager),
        }
    }
}

impl<R, Rend, E, I> AppState<R, Rend, E, I>
where
    R: StatisticRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    pub fn new(
        statistic_repository: Arc<R>,
        event_publisher: Arc<E>,
        renderer: Arc<Rend>,
        integration_service: Arc<I>,
        session_manager: Arc<SessionManager<R>>,
    ) -> Self {
        Self {
            statistic_repository,
            event_publisher,
            renderer,
            integration_service,
            session_manager,
        }
    }
}
