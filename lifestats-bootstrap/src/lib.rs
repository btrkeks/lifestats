use std::sync::Arc;

use anyhow::Result;
use events::event_port::{Event, EventPublisher};
use lifestats_application::use_cases::integration::{
    AnkiBundleFactory, BundleFactory, IntegrationService,
};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_persistence::{
    pool::get_database_pool, repository::SqliteStatisticRepository, SqlitePlotRepository,
};
use lifestats_plotting::PlottersRenderer;

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
    pub plot_repository: Arc<SqlitePlotRepository>,
    pub event_publisher: Arc<NoOpEventPublisher>,
    pub renderer: Arc<PlottersRenderer>,
    pub integration_service: Arc<IntegrationService>,
}

/// The main builder function. This is our Composition Root.
pub fn build_services() -> Result<Application> {
    dotenvy::dotenv().ok();

    // 1. Instantiate concrete infrastructure adapters
    let pool = get_database_pool();
    let statistic_repo = Arc::new(SqliteStatisticRepository::new(Arc::clone(&pool)));
    let plot_repo = Arc::new(SqlitePlotRepository::new(Arc::clone(&pool)));
    let renderer = Arc::new(PlottersRenderer::new());
    let publisher = Arc::new(NoOpEventPublisher);

    let factories: Vec<Arc<dyn BundleFactory>> = vec![Arc::new(AnkiBundleFactory)];
    let integration_service = Arc::new(IntegrationService::new(
        statistic_repo.clone() as Arc<dyn StatisticRepository + Send + Sync>,
        factories,
    ));

    // 3. Return the container with both repositories (for web_api) and services (for logger apps)
    Ok(Application {
        statistic_repository: statistic_repo,
        plot_repository: plot_repo,
        event_publisher: publisher,
        renderer,
        integration_service,
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
