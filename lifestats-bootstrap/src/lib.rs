use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Result;
use chrono_tz::Tz;
use events::event_port::{Event, EventPublisher};
use lifestats_application::integrations::fitbit::ports::{
    FitbitDataApi, FitbitMetric, FitbitOAuthApi, TokenStorePort,
};
use lifestats_application::statistic::ports::TagQueryableStatisticRepository;
use lifestats_application::time::Clock;
use lifestats_application::use_cases::{
    commands::sync_fitbit_data::SyncFitbitDataCommandHandler,
    integration::{
        AnkiBundleFactory, BundleFactory, EbookBundleFactory, EbookEventStore,
        EbookProgressService, EbookProgressServiceTrait, FitbitBundleFactory, IntegrationService,
        LiftLogBundleFactory,
    },
};
use lifestats_application::SessionManager;
use lifestats_core::progress::ports::progress_repository::ProgressRepository;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_fitbit_client::{HttpFitbitDataClient, HttpFitbitOAuthClient};
use lifestats_persistence::{
    ebook_event_store::SqliteEbookEventStore, pool::get_database_pool,
    progress_repository::SqliteProgressRepository, repository::SqliteStatisticRepository,
    token_store::SqliteTokenStore,
};
use lifestats_plotting::PlottersRenderer;
use lifestats_system_clock::SystemClock;

/// Simple no-op event publisher used by default in the bootstrap crate.
#[derive(Clone, Default)]
pub struct NoOpEventPublisher;

impl EventPublisher for NoOpEventPublisher {
    fn publish_event_async(&self, _event: Event) {}

    fn publish_event_sync(&self, _event: Event) -> Result<()> {
        Ok(())
    }
}

/// A container for all the fully constructed application services and infrastructure.
pub struct Application {
    // Direct infrastructure access for web_api
    pub statistic_repository: Arc<SqliteStatisticRepository>,
    pub progress_repository: Arc<dyn ProgressRepository + Send + Sync>,
    pub event_publisher: Arc<NoOpEventPublisher>,
    pub renderer: Arc<PlottersRenderer>,
    pub integration_service: Arc<IntegrationService>,

    // Session management
    pub session_manager: Arc<SessionManager<SqliteStatisticRepository>>,

    // Fitbit integration components
    pub fitbit_oauth_api: Arc<dyn FitbitOAuthApi>,
    pub fitbit_data_api: Arc<dyn FitbitDataApi>,
    pub fitbit_token_store: Arc<dyn TokenStorePort>,
    pub fitbit_clock: Arc<dyn Clock>,
    pub fitbit_sync_handler: Arc<SyncFitbitDataCommandHandler>,
}

/// The main builder function. This is our Composition Root.
pub fn build_services() -> Result<Application> {
    dotenvy::dotenv().ok();

    // 1. Instantiate concrete infrastructure adapters
    let pool = get_database_pool();
    let statistic_repo = Arc::new(SqliteStatisticRepository::new(Arc::clone(&pool)));
    let progress_repo: Arc<dyn ProgressRepository + Send + Sync> =
        Arc::new(SqliteProgressRepository::new(Arc::clone(&pool)));
    let renderer = Arc::new(PlottersRenderer::new());
    let publisher = Arc::new(NoOpEventPublisher);

    let factories: Vec<Arc<dyn BundleFactory>> = vec![
        Arc::new(AnkiBundleFactory),
        Arc::new(EbookBundleFactory),
        Arc::new(FitbitBundleFactory),
        Arc::new(LiftLogBundleFactory),
    ];
    let ebook_event_store: Arc<dyn EbookEventStore> =
        Arc::new(SqliteEbookEventStore::new(Arc::clone(&pool)));
    let ebook_progress_service: Arc<dyn EbookProgressServiceTrait> =
        Arc::new(EbookProgressService::new(Arc::clone(&progress_repo)));
    let integration_service = Arc::new(IntegrationService::new(
        statistic_repo.clone() as Arc<dyn StatisticRepository + Send + Sync>,
        factories,
        Some(ebook_event_store),
        Some(ebook_progress_service),
    ));

    // 2. Instantiate Fitbit integration components
    let http_client = reqwest::Client::new();

    // Load Fitbit configuration from environment
    let fitbit_client_id =
        std::env::var("FITBIT_CLIENT_ID").unwrap_or_else(|_| "dummy_client_id".to_string());
    let fitbit_client_secret = std::env::var("FITBIT_CLIENT_SECRET").ok();
    let fitbit_redirect_uri = std::env::var("FITBIT_REDIRECT_URI")
        .unwrap_or_else(|_| "http://localhost:3000/integrations/fitbit/auth/callback".to_string());

    // Instantiate Fitbit HTTP clients
    let fitbit_oauth_api: Arc<dyn FitbitOAuthApi> = Arc::new(HttpFitbitOAuthClient::new(
        http_client.clone(),
        fitbit_client_id,
        fitbit_client_secret,
        fitbit_redirect_uri,
    ));

    let fitbit_data_api: Arc<dyn FitbitDataApi> =
        Arc::new(HttpFitbitDataClient::new(http_client.clone()));

    // Instantiate token store
    let fitbit_token_store: Arc<dyn TokenStorePort> =
        Arc::new(SqliteTokenStore::new(Arc::clone(&pool)));

    // Instantiate clock
    let fitbit_clock: Arc<dyn Clock> = Arc::new(SystemClock);

    // Parse timezone (default to UTC if not set)
    let timezone: Tz = std::env::var("TIMEZONE")
        .ok()
        .and_then(|tz_str| tz_str.parse().ok())
        .unwrap_or(chrono_tz::UTC);

    // Parse enabled metrics (default to steps and sleep_duration)
    // Use FromStr to make FitbitMetric enum the single source of truth
    let enabled_metrics: HashSet<FitbitMetric> = std::env::var("FITBIT_ENABLED_METRICS")
        .ok()
        .map(|s| {
            s.split(',')
                .filter_map(|m| m.trim().parse::<FitbitMetric>().ok())
                .collect()
        })
        .unwrap_or_else(|| {
            let mut set = HashSet::new();
            set.insert(FitbitMetric::Steps);
            set.insert(FitbitMetric::SleepDuration);
            set
        });

    // Instantiate sync command handler
    let fitbit_sync_handler = Arc::new(SyncFitbitDataCommandHandler::new(
        Arc::clone(&fitbit_oauth_api),
        Arc::clone(&fitbit_data_api),
        Arc::clone(&fitbit_token_store),
        statistic_repo.clone() as Arc<dyn TagQueryableStatisticRepository>,
        statistic_repo.clone() as Arc<dyn StatisticRepository + Send + Sync>,
        Arc::clone(&fitbit_clock),
        timezone,
        enabled_metrics,
    ));

    // Instantiate session manager for heartbeat session tracking
    let session_manager = Arc::new(SessionManager::new(Arc::clone(&statistic_repo)));

    // 3. Return the container with repositories (for web_api) and services (for logger apps)
    Ok(Application {
        statistic_repository: statistic_repo,
        progress_repository: progress_repo,
        event_publisher: publisher,
        renderer,
        integration_service,
        session_manager,
        fitbit_oauth_api,
        fitbit_data_api,
        fitbit_token_store,
        fitbit_clock,
        fitbit_sync_handler,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_services_produces_components() {
        // Note: this test requires DATABASE_URL to be set, but
        // the bootstrap is meant to be used in integration binaries
        // where the environment is configured. Skip if missing.
        if std::env::var("DATABASE_URL").is_err() {
            return;
        }

        let app = build_services().expect("bootstrap should construct services");
        assert!(Arc::strong_count(&app.integration_service) > 0);
        assert!(Arc::strong_count(&app.statistic_repository) > 0);
    }
}
