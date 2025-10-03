mod dto;
mod error;
mod routes;
mod state;

use std::sync::Arc;

use anyhow::Result;
use events::event_port::{Event, EventPublisher};
use lifestats_application::use_cases::{
    goal_management_service::GoalManagementService, logging_service::LoggingService,
    statistic_query_service::StatisticQueryService,
};
use lifestats_persistence::pool::get_database_pool;
use lifestats_persistence::repository::SqliteStatisticRepository;
use state::AppState;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

/// Simple no-op event publisher for when we don't need event processing
#[derive(Clone)]
struct NoOpEventPublisher;

impl EventPublisher for NoOpEventPublisher {
    fn publish_event_async(&self, _event: Event) {
        // Do nothing
    }

    fn publish_event_sync(&self, _event: Event) -> Result<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    // Initialize tracing subscriber for structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,web_api=debug,tower_http=debug".into()),
        )
        .init();

    tracing::info!("Starting lifestats-web-api");

    // Create database connection pool
    let pool = get_database_pool();

    // Instantiate concrete adapters
    let repo = Arc::new(SqliteStatisticRepository::new(pool));
    let publisher = Arc::new(NoOpEventPublisher);

    // Instantiate application services
    let logging_service = Arc::new(LoggingService::new(
        Arc::clone(&repo),
        Arc::clone(&publisher),
    ));
    let goal_service = Arc::new(GoalManagementService::new(
        Arc::clone(&repo),
        Arc::clone(&publisher),
    ));
    let query_service = Arc::new(StatisticQueryService::new(Arc::clone(&repo)));

    // Create shared application state
    let app_state = AppState::new(logging_service, goal_service, query_service);

    // Build the router with all routes
    let app = routes::create_router(app_state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    // Bind to address and start server
    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("Listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
