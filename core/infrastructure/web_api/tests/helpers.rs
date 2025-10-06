use anyhow::Result;
use chrono::NaiveDate;
use events::event_port::{Event, EventPublisher};
use lifestats_application::use_cases::{
    goal_management_service::GoalManagementService,
    integration::{AnkiBundleFactory, BundleFactory, IntegrationService},
    logging_service::LoggingService,
    plotting_service::PlottingService,
    statistic_query_service::StatisticQueryService,
};
use lifestats_core::shared::ids::{PlotId, StatisticId};
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::error::RepositoryError;
use lifestats_core::statistic::ports::statistic_repository::{
    MockStatisticRepository, StatisticRepository,
};
use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
use lifestats_web_api::state::AppState;
use mockall::mock;
use plot_domain::plot::data::PlotData;
use plot_domain::plot::renderer::{PlotRenderer, RenderConfig};
use plot_domain::plot::repository::PlotRepository;
use plot_domain::plot::specification::PlotSpecification;
use std::sync::Arc;

/// Simple no-op event publisher for testing
#[derive(Clone)]
pub struct NoOpEventPublisher;

impl EventPublisher for NoOpEventPublisher {
    fn publish_event_async(&self, _event: Event) {
        // Do nothing in tests
    }

    fn publish_event_sync(&self, _event: Event) -> Result<()> {
        Ok(())
    }
}

// Mock plot repository and renderer for tests
mock! {
    pub PlotRepo {}

    impl Clone for PlotRepo {
        fn clone(&self) -> Self;
    }

    impl PlotRepository for PlotRepo {
        fn find_by_id(&self, id: PlotId) -> Result<Option<PlotSpecification>, RepositoryError>;
        fn find_all(&self) -> Result<Vec<PlotSpecification>, RepositoryError>;
        fn find_by_statistic_id(&self, statistic_id: StatisticId) -> Result<Vec<PlotSpecification>, RepositoryError>;
        fn save(&self, plot_spec: &PlotSpecification) -> Result<(), RepositoryError>;
        fn delete(&self, id: PlotId) -> Result<(), RepositoryError>;
    }
}

mock! {
    pub PlotRend {}

    impl Clone for PlotRend {
        fn clone(&self) -> Self;
    }

    impl PlotRenderer for PlotRend {
        fn render(&self, data: &PlotData, config: &RenderConfig) -> Result<Vec<u8>>;
    }
}

/// Create an AppState with a mock repository for testing
#[allow(dead_code)]
pub fn mock_app_state(
    mock_repo: Arc<MockStatisticRepository>,
) -> AppState<MockStatisticRepository, MockPlotRepo, MockPlotRend, NoOpEventPublisher> {
    let publisher = Arc::new(NoOpEventPublisher);
    let plot_repo = Arc::new(MockPlotRepo::new());
    let renderer = Arc::new(MockPlotRend::new());

    let logging_service = Arc::new(LoggingService::new(
        Arc::clone(&mock_repo),
        Arc::clone(&publisher),
    ));
    let goal_service = Arc::new(GoalManagementService::new(
        Arc::clone(&mock_repo),
        Arc::clone(&publisher),
    ));
    let query_service = Arc::new(StatisticQueryService::new(Arc::clone(&mock_repo)));
    let plotting_service = Arc::new(PlottingService::new(
        Arc::clone(&mock_repo),
        plot_repo,
        renderer,
    ));

    // Create integration service
    let factories: Vec<Arc<dyn BundleFactory>> = vec![Arc::new(AnkiBundleFactory)];
    let integration_service = Arc::new(IntegrationService::new(
        mock_repo.clone() as Arc<dyn StatisticRepository + Send + Sync>,
        factories,
    ));

    AppState::new(
        logging_service,
        goal_service,
        query_service,
        plotting_service,
        integration_service,
    )
}

/// Factory function to create a mock IntegerAmount statistic
#[allow(dead_code)]
pub fn mock_integer_statistic(id: u32, name: &str, category: &str) -> AnyStatistic {
    let metadata = StatisticMetadata::new(
        StatisticId(id),
        category.to_string().into_boxed_str(),
        name.to_string().into_boxed_str(),
    );
    AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
        metadata,
        None,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    ))
}
