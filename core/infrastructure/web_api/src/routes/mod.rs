pub mod daily_summary;
pub mod dashboard;
pub mod goals;
pub mod integrations;
pub mod plots;
pub mod progress;
pub mod sessions;
pub mod statistics;

use axum::{
    routing::{get, patch, post, put},
    Router,
};
use events::event_port::EventPublisher;
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use plot_domain::plot::renderer::PlotRenderer;

use crate::state::AppState;

/// Create and configure the application router with all routes
pub fn create_router<R, Rend, E, I>(app_state: AppState<R, Rend, E, I>) -> Router
where
    R: StatisticRepository + Send + Sync + 'static,
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
            "/statistics/loggable",
            get(statistics::get_loggable_statistics),
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
            "/statistics/{id}/logged/{date}",
            get(statistics::get_logged_value),
        )
        .route(
            "/statistics/{id}/time_spans/total",
            get(statistics::get_time_spans_total),
        )
        .route(
            "/statistics/{id}/integer_amount/total",
            get(statistics::get_integer_amount_total),
        )
        .route(
            "/statistics/{id}/consistency",
            get(statistics::get_consistency_score),
        )
        .route(
            "/statistics/{id}/score",
            get(statistics::get_statistic_score),
        )
        .route(
            "/statistics/{id}/achievements",
            get(statistics::get_daily_achievements),
        )
        .route(
            "/statistics/{id}/performance",
            get(statistics::get_goal_performance),
        )
        .route("/statistics/{id}/streak", get(statistics::get_streak))
        // Progress item endpoints
        .route(
            "/progress-items",
            post(progress::create_progress_item).get(progress::list_progress_items),
        )
        .route("/progress-items/{id}", get(progress::get_progress_item))
        .route(
            "/progress-items/{id}/target",
            patch(progress::update_progress_target),
        )
        .route(
            "/progress-items/{id}/checkpoints",
            post(progress::log_progress_checkpoint).get(progress::get_progress_checkpoints),
        )
        .route(
            "/progress-items/{id}/summary",
            get(progress::get_progress_summary),
        )
        .route(
            "/progress-items/{id}/daily-deltas",
            get(progress::get_progress_daily_deltas),
        )
        // Dashboard endpoint
        .route("/statistics/dashboard", post(dashboard::get_dashboard))
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
        // Plot endpoints (stateless)
        .route(
            "/statistics/{id}/plot/image",
            get(plots::get_statistic_plot_image),
        )
        .route(
            "/statistics/{id}/plot/data",
            get(plots::get_statistic_plot_data),
        )
        // Timeline plot endpoint
        .route(
            "/plots/timeline/{date}",
            get(plots::get_daily_timeline_plot),
        )
        // Session endpoints
        .route("/sessions", get(sessions::list_sessions))
        .route(
            "/sessions/{statistic_id}/start",
            post(sessions::start_session),
        )
        .route(
            "/sessions/{statistic_id}/beat",
            post(sessions::send_heartbeat),
        )
        .route(
            "/sessions/{statistic_id}/stop",
            post(sessions::stop_session),
        )
        .route(
            "/sessions/{statistic_id}",
            get(sessions::get_session_status),
        )
        // Integration endpoints
        .route(
            "/integrations/ebook/events",
            post(integrations::ingest_ebook_event),
        )
        .route(
            "/integrations/liftlog/days",
            put(integrations::sync_liftlog_days),
        )
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
        // Daily Summary endpoint
        .route(
            "/daily_summary/{date}",
            get(daily_summary::get_daily_summary),
        )
        .with_state(app_state)
}
