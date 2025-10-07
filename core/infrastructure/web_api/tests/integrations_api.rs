mod helpers;

use axum_test::TestServer;
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_core::shared::ids::StatisticId;
use lifestats_web_api::routes::create_router;
use lifestats_web_api::state::AppState;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(test)]
use mockall::mock;

use crate::helpers::{MockPlotRend, MockPlotRepo, NoOpEventPublisher};
use lifestats_application::use_cases::{
    goal_management_service::GoalManagementService, logging_service::LoggingService,
    plotting_service::PlottingService, statistic_management_service::StatisticManagementService,
    statistic_query_service::StatisticQueryService,
};
use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;

// Create a mock for IntegrationServiceTrait
mock! {
    pub IntegrationSvc {}

    impl IntegrationServiceTrait for IntegrationSvc {
        fn provision_bundle(
            &self,
            source: &str,
            context: HashMap<String, String>,
        ) -> lifestats_application::error::ApplicationResult<HashMap<String, StatisticId>>;
    }
}

/// Helper function to create an AppState with a mock integration service
fn mock_app_state_with_integrations(
    mock_repo: Arc<MockStatisticRepository>,
    mock_integration_service: Arc<MockIntegrationSvc>,
) -> AppState<
    MockStatisticRepository,
    MockPlotRepo,
    MockPlotRend,
    NoOpEventPublisher,
    MockIntegrationSvc,
> {
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
    let management_service = Arc::new(StatisticManagementService::new(
        Arc::clone(&mock_repo),
        Arc::clone(&publisher),
    ));
    let query_service = Arc::new(StatisticQueryService::new(Arc::clone(&mock_repo)));
    let plotting_service = Arc::new(PlottingService::new(
        Arc::clone(&mock_repo),
        plot_repo,
        renderer,
    ));

    AppState::new(
        logging_service,
        goal_service,
        management_service,
        query_service,
        plotting_service,
        mock_integration_service,
    )
}

// ============================================================================
// POST /integrations/anki Contract Tests
// ============================================================================

#[tokio::test]
async fn test_provision_anki_bundle_contract() {
    // 1. ARRANGE
    // Create a mock IntegrationService
    let mut mock_integration_service = MockIntegrationSvc::new();

    // Set the expectation: when `provision_bundle` is called with "anki",
    // return a predictable HashMap with our internal, logical keys.
    mock_integration_service
        .expect_provision_bundle()
        .withf(|source, _context| source == "anki") // Match only calls for "anki"
        .times(1)
        .returning(|_, _| {
            let mut map = HashMap::new();
            map.insert("time_spent".to_string(), StatisticId(38));
            map.insert("disruptions".to_string(), StatisticId(39));
            map.insert("backlog".to_string(), StatisticId(40));
            Ok(map)
        });

    // Create the app state with our mock service
    let mock_repo = MockStatisticRepository::new();
    let app_state =
        mock_app_state_with_integrations(Arc::new(mock_repo), Arc::new(mock_integration_service));
    let server = TestServer::new(create_router(app_state)).unwrap();

    // 2. ACT
    // Call the endpoint just like the Anki client would
    let response = server
        .post("/integrations/anki")
        .json(&json!({ "deck_name": "Test Deck" }))
        .await;

    // 3. ASSERT
    // Verify the contract is met
    assert_eq!(response.status_code(), 200);

    // Parse the JSON body
    let body: serde_json::Value = response.json();

    // Assert that the keys are exactly what the client expects ("_id" suffix)
    // This is the core of our contract test.
    assert_eq!(body["time_spent_id"], 38);
    assert_eq!(body["disruptions_id"], 39);
    assert_eq!(body["backlog_id"], 40);

    // Also assert that the old, incorrect keys are NOT present
    assert!(body.get("time_spent").is_none());
    assert!(body.get("disruptions").is_none());
    assert!(body.get("backlog").is_none());
}

#[tokio::test]
async fn test_provision_anki_bundle_preserves_keys() {
    // This test ensures we don't accidentally change the key transformation logic
    let mut mock_integration_service = MockIntegrationSvc::new();

    mock_integration_service
        .expect_provision_bundle()
        .withf(|source, _| source == "anki")
        .times(1)
        .returning(|_, _| {
            let mut map = HashMap::new();
            // Return keys that should be transformed
            map.insert("custom_key".to_string(), StatisticId(100));
            Ok(map)
        });

    let mock_repo = MockStatisticRepository::new();
    let app_state =
        mock_app_state_with_integrations(Arc::new(mock_repo), Arc::new(mock_integration_service));
    let server = TestServer::new(create_router(app_state)).unwrap();

    let response = server
        .post("/integrations/anki")
        .json(&json!({ "deck_name": "Test" }))
        .await;

    assert_eq!(response.status_code(), 200);

    let body: serde_json::Value = response.json();

    // Verify the key transformation adds "_id" suffix
    assert_eq!(body["custom_key_id"], 100);
    assert!(body.get("custom_key").is_none());
}
