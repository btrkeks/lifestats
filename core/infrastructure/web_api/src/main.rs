mod dto;
mod error;
mod helpers;
mod integrations;
mod routes;
mod state;

use std::sync::Arc;

use anyhow::Result;
use lifestats_bootstrap::build_services;
use state::AppState;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::integrations::fitbit::config::FitbitConfig;

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

    let app = build_services()?;

    // Build lean AppState with only globally shared dependencies
    // Note: PlotRepository removed - plotting is now stateless
    let app_state = AppState::new(
        app.statistic_repository,
        app.progress_repository,
        app.event_publisher,
        app.renderer,
        app.integration_service,
        app.session_manager,
    );

    // Load Fitbit configuration
    let fitbit_config = load_fitbit_config();
    let fitbit_state_cache =
        Arc::new(crate::integrations::fitbit::state_cache::OAuthStateCache::new());

    // Build router with state and inject Fitbit dependencies via Extension layers
    let app = routes::create_router(app_state)
        .layer(axum::Extension(fitbit_config))
        .layer(axum::Extension(app.fitbit_oauth_api))
        .layer(axum::Extension(app.fitbit_token_store))
        .layer(axum::Extension(fitbit_state_cache))
        .layer(axum::Extension(app.fitbit_sync_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!("Listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

fn load_fitbit_config() -> Arc<FitbitConfig> {
    // Load Fitbit configuration from environment
    let fitbit_client_id =
        std::env::var("FITBIT_CLIENT_ID").unwrap_or_else(|_| "dummy_client_id".to_string());
    let fitbit_client_secret =
        std::env::var("FITBIT_CLIENT_SECRET").unwrap_or_else(|_| "dummy_secret".to_string());
    let fitbit_redirect_uri = std::env::var("FITBIT_REDIRECT_URI")
        .unwrap_or_else(|_| "http://localhost:3000/integrations/fitbit/auth/callback".to_string());
    let fitbit_scopes = std::env::var("FITBIT_SCOPES")
        .unwrap_or_else(|_| "activity sleep".to_string())
        .split_whitespace()
        .map(String::from)
        .collect();

    Arc::new(FitbitConfig::new(
        fitbit_client_id,
        fitbit_client_secret,
        fitbit_redirect_uri,
        fitbit_scopes,
    ))
}
