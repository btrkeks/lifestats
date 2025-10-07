use std::sync::Arc;

use anyhow::Result;
use events::event_port::{Event, EventPublisher};
use lifestats_application::use_cases::{
    goal_management_service::GoalManagementService,
    integration::{AnkiBundleFactory, BundleFactory, IntegrationService},
    logging_service::LoggingService,
    plotting_service::PlottingService,
    statistic_management_service::StatisticManagementService,
    statistic_query_service::StatisticQueryService,
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

/// A container for all the fully constructed application services.
pub struct Application {
    pub logging_service: Arc<LoggingService<SqliteStatisticRepository, NoOpEventPublisher>>,
    pub goal_service: Arc<GoalManagementService<SqliteStatisticRepository, NoOpEventPublisher>>,
    pub management_service:
        Arc<StatisticManagementService<SqliteStatisticRepository, NoOpEventPublisher>>,
    pub query_service: Arc<StatisticQueryService<SqliteStatisticRepository>>,
    pub plotting_service:
        Arc<PlottingService<SqliteStatisticRepository, SqlitePlotRepository, PlottersRenderer>>,
    pub integration_service: Arc<IntegrationService>,
    pub statistic_repository: Arc<SqliteStatisticRepository>,
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

    // 2. Instantiate application services with the concrete adapters
    let logging_service = Arc::new(LoggingService::new(
        Arc::clone(&statistic_repo),
        Arc::clone(&publisher),
    ));
    let goal_service = Arc::new(GoalManagementService::new(
        Arc::clone(&statistic_repo),
        Arc::clone(&publisher),
    ));
    let management_service = Arc::new(StatisticManagementService::new(
        Arc::clone(&statistic_repo),
        Arc::clone(&publisher),
    ));
    let query_service = Arc::new(StatisticQueryService::new(Arc::clone(&statistic_repo)));
    let plotting_service = Arc::new(PlottingService::new(
        Arc::clone(&statistic_repo),
        plot_repo,
        renderer,
    ));

    let factories: Vec<Arc<dyn BundleFactory>> = vec![Arc::new(AnkiBundleFactory)];
    let integration_service = Arc::new(IntegrationService::new(
        statistic_repo.clone() as Arc<dyn StatisticRepository + Send + Sync>,
        factories,
    ));

    // 3. Return the container of all assembled services
    Ok(Application {
        logging_service,
        goal_service,
        management_service,
        query_service,
        plotting_service,
        integration_service,
        statistic_repository: statistic_repo,
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
        assert!(Arc::strong_count(&app.logging_service) > 0);
        assert!(Arc::strong_count(&app.goal_service) > 0);
        assert!(Arc::strong_count(&app.management_service) > 0);
        assert!(Arc::strong_count(&app.query_service) > 0);
        assert!(Arc::strong_count(&app.plotting_service) > 0);
        assert!(Arc::strong_count(&app.integration_service) > 0);
        assert!(Arc::strong_count(&app.statistic_repository) > 0);
    }
}
