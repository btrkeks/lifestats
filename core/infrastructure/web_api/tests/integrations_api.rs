mod helpers;

use axum_test::TestServer;
use chrono::NaiveDate;
use lifestats_application::use_cases::integration::IntegrationServiceTrait;
use lifestats_application::SessionManager;
use lifestats_core::shared::ids::StatisticId;
use lifestats_web_api::routes::create_router;
use lifestats_web_api::state::AppState;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(test)]
use mockall::mock;

use crate::helpers::{MockPlotRend, NoOpEventPublisher};
use lifestats_core::progress::ports::progress_repository::MockProgressRepository;
use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;

// Create a mock for IntegrationServiceTrait
mock! {
    pub IntegrationSvc {}

    impl IntegrationServiceTrait for IntegrationSvc {
        fn replace_beurer_day(&self, day: NaiveDate, reading: lifestats_core::beurer::BeurerDayReading) -> Result<lifestats_core::beurer::BeurerDayReplacement, lifestats_core::beurer::BeurerDayError>;
        fn ebook_server_id(&self) -> Result<String, lifestats_core::ebook::EbookError>;
        fn record_ebook_day(&self, producer: lifestats_core::ebook::ProducerId, book: lifestats_core::ebook::BookKey, day: NaiveDate, report: lifestats_core::ebook::EbookDayReport) -> Result<lifestats_core::ebook::EbookWriteOutcome, lifestats_core::ebook::EbookError>;

        fn replace_yomikata_day(&self, day: NaiveDate, totals: lifestats_core::yomikata::YomikataDayTotals) -> Result<lifestats_core::yomikata::YomikataDayReplacement,lifestats_core::yomikata::YomikataDayError>;

        fn provision_bundle(
            &self,
            source: &str,
            context: HashMap<String, String>,
        ) -> lifestats_application::error::ApplicationResult<HashMap<String, StatisticId>>;


        fn sync_liftlog_days(
            &self,
            days: std::collections::BTreeMap<
                NaiveDate,
                Vec<lifestats_application::use_cases::integration::LiftLogWorkout>,
            >,
        ) -> lifestats_application::error::ApplicationResult<
            lifestats_application::use_cases::integration::LiftLogDaysSyncResult,
        >;
    }
}

/// Helper function to create an AppState with a mock integration service
fn mock_app_state_with_integrations(
    mock_repo: Arc<MockStatisticRepository>,
    mock_integration_service: Arc<MockIntegrationSvc>,
) -> AppState<MockStatisticRepository, MockPlotRend, NoOpEventPublisher, MockIntegrationSvc> {
    let publisher = Arc::new(NoOpEventPublisher);
    let progress_repo = Arc::new(MockProgressRepository::new());
    let renderer = Arc::new(MockPlotRend::new());

    let session_manager = Arc::new(SessionManager::new(mock_repo.clone()));

    AppState::new(
        mock_repo,
        progress_repo,
        publisher,
        renderer,
        mock_integration_service,
        session_manager,
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
            map.insert("finished_time".to_string(), StatisticId(41));
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
    assert_eq!(body["finished_time_id"], 41);

    // Also assert that the old, incorrect keys are NOT present
    assert!(body.get("time_spent").is_none());
    assert!(body.get("disruptions").is_none());
    assert!(body.get("backlog").is_none());
    assert!(body.get("finished_time").is_none());
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
