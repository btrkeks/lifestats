pub mod integrations;
pub mod plots;
pub mod statistics;

use axum::{routing::get, routing::post, Router};
use events::event_port::EventPublisher;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use plot_domain::plot::renderer::PlotRenderer;
use plot_domain::plot::repository::PlotRepository;

use crate::state::AppState;

/// Create and configure the application router with all routes
pub fn create_router<R, P, Rend, E>(app_state: AppState<R, P, Rend, E>) -> Router
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
{
    Router::new()
        .route("/statistics", get(statistics::get_all_statistics))
        .route("/statistics/:id/log/:date", post(statistics::log_value))
        .route(
            "/statistics/:id/log_time_frame/:date",
            post(statistics::log_time_frame),
        )
        .route("/plots/:id/image", get(plots::get_plot_image))
        .route(
            "/integrations/:source",
            post(integrations::provision_integration),
        )
        .with_state(app_state)
}
