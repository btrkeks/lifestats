mod dto;
mod error;
mod helpers;
mod routes;
mod state;

use anyhow::Result;
use lifestats_bootstrap::build_services;
use state::AppState;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,web_api=debug,tower_http=debug".into()),
        )
        .init();

    tracing::info!("Starting lifestats-web-api");

    if let Ok(database_url) = std::env::var("DATABASE_URL") {
        tracing::info!("Using database: {}", database_url);
    } else {
        tracing::warn!("DATABASE_URL not set");
    }

    let app = build_services()?;

    let app_state = AppState::new(
        app.statistic_repository,
        app.plot_repository,
        app.event_publisher,
        app.renderer,
        app.integration_service,
    );

    let app = routes::create_router(app_state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("Listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
