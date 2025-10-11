pub mod goals;
pub mod integrations;
pub mod plots;
pub mod statistics;

use axum::{
    routing::{get, post},
    Router,
};
use events::event_port::EventPublisher;
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use plot_domain::plot::renderer::PlotRenderer;
use plot_domain::plot::repository::PlotRepository;

use crate::state::AppState;

/// Create and configure the application router with all routes
pub fn create_router<R, P, Rend, E, I>(app_state: AppState<R, P, Rend, E, I>) -> Router
where
    R: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    Rend: PlotRenderer + 'static,
    E: EventPublisher + 'static,
    I: IntegrationServiceTrait + 'static,
{
    Router::new()
        // Statistics endpoints
        .route(
            "/statistics",
            get(statistics::get_all_statistics).post(statistics::create_statistic),
        )
        .route(
            "/statistics/{id}",
            get(statistics::get_statistic).delete(statistics::delete_statistic),
        )
        .route("/statistics/{id}/log/{date}", post(statistics::log_value))
        .route(
            "/statistics/{id}/log_time_frame/{date}",
            post(statistics::log_time_frame),
        )
        .route(
            "/statistics/{id}/time_spans/total",
            get(statistics::get_time_spans_total),
        )
        // Goal management endpoints
        .route(
            "/statistics/{id}/goals/default",
            get(goals::get_default_goal)
                .put(goals::set_default_goal)
                .delete(goals::delete_default_goal),
        )
        .route(
            "/statistics/{id}/goals/{date}",
            get(goals::get_daily_goal)
                .put(goals::set_daily_goal)
                .delete(goals::delete_daily_goal),
        )
        // Plot endpoints
        .route("/plots/{id}/image", get(plots::get_plot_image))
        .route("/plots/{id}/data", get(plots::get_plot_data))
        // Integration endpoints
        .route(
            "/integrations/{source}",
            post(integrations::provision_integration),
        )
        // Fitbit OAuth & Sync endpoints
        .route(
            "/integrations/fitbit/auth/begin",
            get(crate::integrations::fitbit::handlers::auth_begin_handler),
        )
        .route(
            "/integrations/fitbit/auth/callback",
            get(crate::integrations::fitbit::handlers::auth_callback_handler),
        )
        .route(
            "/integrations/fitbit/sync",
            post(crate::integrations::fitbit::handlers::sync_handler),
        )
        .with_state(app_state)
}
