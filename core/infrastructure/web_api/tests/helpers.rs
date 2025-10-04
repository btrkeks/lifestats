use anyhow::Result;
use chrono::NaiveDate;
use events::event_port::{Event, EventPublisher};
use lifestats_application::use_cases::{
    goal_management_service::GoalManagementService, logging_service::LoggingService,
    statistic_query_service::StatisticQueryService,
};
use lifestats_domain::shared::ids::StatisticId;
use lifestats_domain::statistic::any_statistic::AnyStatistic;
use lifestats_domain::statistic::ports::statistic_repository::MockStatisticRepository;
use lifestats_domain::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
use lifestats_domain::statistic::value_objects::statistic_metadata::StatisticMetadata;
use lifestats_web_api::state::AppState;
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

/// Create an AppState with a mock repository for testing
#[allow(dead_code)]
pub fn mock_app_state(
    mock_repo: Arc<MockStatisticRepository>,
) -> AppState<MockStatisticRepository, NoOpEventPublisher> {
    let publisher = Arc::new(NoOpEventPublisher);

    let logging_service = Arc::new(LoggingService::new(
        Arc::clone(&mock_repo),
        Arc::clone(&publisher),
    ));
    let goal_service = Arc::new(GoalManagementService::new(
        Arc::clone(&mock_repo),
        Arc::clone(&publisher),
    ));
    let query_service = Arc::new(StatisticQueryService::new(Arc::clone(&mock_repo)));

    AppState::new(logging_service, goal_service, query_service)
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
