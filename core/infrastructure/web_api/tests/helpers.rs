use anyhow::Result;
use chrono::NaiveDate;
use events::event_port::{Event, EventPublisher};
use lifestats_application::use_cases::integration::{
    AnkiBundleFactory, BundleFactory, EbookBundleFactory, IntegrationService,
};
use lifestats_application::SessionManager;
use lifestats_core::shared::ids::StatisticId;
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

// Mock plot renderer for tests
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
) -> AppState<MockStatisticRepository, MockPlotRend, NoOpEventPublisher, IntegrationService> {
    let publisher = Arc::new(NoOpEventPublisher);
    let renderer = Arc::new(MockPlotRend::new());

    let factories: Vec<Arc<dyn BundleFactory>> =
        vec![Arc::new(AnkiBundleFactory), Arc::new(EbookBundleFactory)];
    let integration_service = Arc::new(IntegrationService::new(
        mock_repo.clone() as Arc<dyn StatisticRepository + Send + Sync>,
        factories,
        None,
    ));

    let session_manager = Arc::new(SessionManager::new(mock_repo.clone()));

    AppState::new(
        mock_repo,
        publisher,
        renderer,
        integration_service,
        session_manager,
    )
}

/// Factory function to create a mock IntegerAmount statistic
#[allow(dead_code)]
pub fn mock_integer_statistic(id: u32, name: &str, category: &str) -> AnyStatistic {
    let metadata = StatisticMetadata::new(StatisticId(id), category.to_string(), name.to_string())
        .with_tag("statistic_type".to_string(), "IntegerAmount".to_string());
    AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
        metadata,
        None,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    ))
}
