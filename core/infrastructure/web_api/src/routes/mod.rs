pub mod statistics;

use axum::{routing::get, routing::post, Router};
use events::event_port::EventPublisher;
use lifestats_domain::statistic::ports::statistic_repository::StatisticRepository;

use crate::state::AppState;

/// Create and configure the application router with all routes
pub fn create_router<R, E>(app_state: AppState<R, E>) -> Router
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    Router::new()
        .route("/statistics", get(statistics::get_all_statistics))
        .route("/statistics/:id/log/:date", post(statistics::log_value))
        .with_state(app_state)
}
