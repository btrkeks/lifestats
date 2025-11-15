mod helpers;

use axum_test::TestServer;
use helpers::{mock_app_state, mock_integer_statistic};
use lifestats_core::statistic::ports::error::RepositoryError;
use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
use lifestats_web_api::dto::statistic::StatisticMetadataResponse;
use lifestats_web_api::routes::create_router;
use std::sync::Arc;

// ============================================================================
// GET /statistics Tests
// ============================================================================

#[tokio::test]
async fn test_get_all_statistics_happy_path() {
    // Create mock repository
    let mut mock_repo = MockStatisticRepository::new();

    // Set expectation: return two mock statistics
    let stat1 = mock_integer_statistic(1, "Push Ups", "Fitness");
    let stat2 = mock_integer_statistic(2, "Water Intake", "Health");
    let stats = vec![stat1, stat2];

    mock_repo
        .expect_find_all_metadata()
        .times(1)
        .returning(move || Ok(stats.clone()));

    // Create app state and router
    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);

    // Create test server
    let server = TestServer::new(app).unwrap();

    // Send request
    let response = server.get("/statistics").await;

    // Assert status code
    assert_eq!(response.status_code(), 200);

    // Assert response body
    let body: Vec<StatisticMetadataResponse> = response.json();
    assert_eq!(body.len(), 2);
    assert_eq!(body[0].id, 1);
    assert_eq!(body[0].name, "Push Ups");
    assert_eq!(body[0].category, "Fitness");
    assert_eq!(body[0].statistic_type, "IntegerAmount");
    assert_eq!(body[1].id, 2);
    assert_eq!(body[1].name, "Water Intake");
    assert_eq!(body[1].category, "Health");
    assert_eq!(body[1].statistic_type, "IntegerAmount");
}

#[tokio::test]
async fn test_get_all_statistics_repository_error() {
    // Create mock repository
    let mut mock_repo = MockStatisticRepository::new();

    // Set expectation: return an error
    mock_repo.expect_find_all_metadata().times(1).returning(|| {
        Err(RepositoryError::OperationFailed(anyhow::anyhow!(
            "Database offline"
        )))
    });

    // Create app state and router
    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);

    // Create test server
    let server = TestServer::new(app).unwrap();

    // Send request
    let response = server.get("/statistics").await;

    // Assert status code
    assert_eq!(response.status_code(), 500);

    // Assert error response
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"], "Internal server error");
}

// ============================================================================
// POST /statistics/:id/log/:date Tests
// ============================================================================

#[tokio::test]
async fn test_log_value_successful() {
    // Create mock repository
    let mut mock_repo = MockStatisticRepository::new();

    let stat = mock_integer_statistic(1, "Push Ups", "Fitness");

    // Set expectations for the logging service workflow
    mock_repo
        .expect_find_metadata_by_id()
        .times(1)
        .returning(move |_| Ok(stat.clone()));

    mock_repo
        .expect_find_log_value()
        .times(1)
        .returning(|_, _| Ok(None));

    mock_repo
        .expect_save_log_value()
        .times(1)
        .returning(|_, _, _| Ok(()));

    // Create app state and router
    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);

    // Create test server
    let server = TestServer::new(app).unwrap();

    // Create valid payload
    let payload = serde_json::json!({
        "value": "10"
    });

    // Send request
    let response = server
        .post("/statistics/1/log/2024-01-01")
        .json(&payload)
        .await;

    // Assert status code
    assert_eq!(response.status_code(), 204);
}

#[tokio::test]
async fn test_log_value_validation_failure() {
    // Create mock repository (no expectations needed - validation fails first)
    let mock_repo = MockStatisticRepository::new();

    // Create app state and router
    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);

    // Create test server
    let server = TestServer::new(app).unwrap();

    // Create INVALID payload (empty value)
    let payload = serde_json::json!({
        "value": ""
    });

    // Send request
    let response = server
        .post("/statistics/1/log/2024-01-01")
        .json(&payload)
        .await;

    // Assert status code (422 for validation errors)
    assert_eq!(response.status_code(), 422);

    // Assert error response contains validation message
    let body: serde_json::Value = response.json();
    assert!(body["error"].as_str().unwrap().contains("value"));
}

#[tokio::test]
async fn test_log_value_statistic_not_found() {
    // Create mock repository
    let mut mock_repo = MockStatisticRepository::new();

    // Set expectation: find_metadata_by_id returns an error
    mock_repo
        .expect_find_metadata_by_id()
        .times(1)
        .returning(|_| Err(RepositoryError::NotFound));

    // Create app state and router
    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);

    // Create test server
    let server = TestServer::new(app).unwrap();

    // Create valid payload
    let payload = serde_json::json!({
        "value": "10"
    });

    // Send request to non-existent statistic
    let response = server
        .post("/statistics/999/log/2024-01-01")
        .json(&payload)
        .await;

    // Assert status code (404 for NotFound errors with new error handling)
    assert_eq!(response.status_code(), 404);

    // Assert error response
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"], "The requested resource was not found.");
}

// ============================================================================
// GET /statistics/:id/performance Tests
// ============================================================================
//
// Note: Detailed business logic testing is covered by unit tests in the
// application layer. These integration tests verify basic endpoint functionality.

#[tokio::test]
async fn test_get_goal_performance_statistic_not_found() {
    // Create mock repository
    let mut mock_repo = MockStatisticRepository::new();

    // Set expectation: find_metadata_by_id returns NotFound
    mock_repo
        .expect_find_metadata_by_id()
        .times(1)
        .returning(|_| Err(RepositoryError::NotFound));

    // Create app state and router
    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);

    // Create test server
    let server = TestServer::new(app).unwrap();

    // Send request
    let response = server
        .get("/statistics/999/performance?start_date=2025-01-01&end_date=2025-01-31")
        .await;

    // Assert status code
    assert_eq!(response.status_code(), 404);

    // Assert error response
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"], "The requested resource was not found.");
}

#[tokio::test]
async fn test_get_goal_performance_invalid_date_format() {
    // Create mock repository (no expectations needed - validation fails first)
    let mock_repo = MockStatisticRepository::new();

    // Create app state and router
    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);

    // Create test server
    let server = TestServer::new(app).unwrap();

    // Send request with invalid date format
    let response = server
        .get("/statistics/1/performance?start_date=2025-13-01&end_date=2025-01-31")
        .await;

    // Assert status code
    assert_eq!(response.status_code(), 400);

    // Assert error response contains validation message
    let body: serde_json::Value = response.json();
    assert!(body["error"].as_str().unwrap().contains("Invalid"));
}
