mod helpers;

use axum_test::TestServer;
use chrono::{Days, NaiveDate, Utc};
use helpers::{mock_app_state, mock_integer_statistic};
use lifestats_core::shared::dated_value::DatedValue;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::error::RepositoryError;
use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
use lifestats_core::statistic::types::logable_statistics::habit::HabitStatistic;
use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
use lifestats_core::statistic::value_objects::goals::amount_goal::AmountGoal;
use lifestats_core::statistic::value_objects::goals::goal_query_result::GoalQueryResult;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
use lifestats_web_api::dto::statistic::StatisticMetadataResponse;
use lifestats_web_api::routes::create_router;
use serde_json::json;
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
async fn test_get_all_statistics_includes_default_goal() {
    let mut mock_repo = MockStatisticRepository::new();

    let with_goal = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
        StatisticMetadata::new(StatisticId(1), "Fitness".into(), "Push Ups".into()),
        Some(AmountGoal::new(10, true)),
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    ));
    let without_goal = mock_integer_statistic(2, "Water Intake", "Health");
    let stats = vec![with_goal, without_goal];

    mock_repo
        .expect_find_all_metadata()
        .times(1)
        .returning(move || Ok(stats.clone()));

    let app = create_router(mock_app_state(Arc::new(mock_repo)));
    let server = TestServer::new(app).unwrap();

    let response = server.get("/statistics").await;
    assert_eq!(response.status_code(), 200);

    let body: serde_json::Value = response.json();
    assert_eq!(
        body[0]["default_goal"],
        json!({"target_amount": 10, "is_at_least": true})
    );
    // Key is present with an explicit null when no default goal is set
    assert!(body[1].get("default_goal").is_some());
    assert_eq!(body[1]["default_goal"], serde_json::Value::Null);
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
        .expect_log_integer_value()
        .times(1)
        .returning(|_, _, _, _| Ok(true));

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
// GET /statistics/:id/integer_amount/total Tests
// ============================================================================

#[tokio::test]
async fn test_get_integer_amount_total_success() {
    let mut mock_repo = MockStatisticRepository::new();
    let stat = mock_integer_statistic(1, "Anki Reps", "Learning");

    mock_repo
        .expect_find_metadata_by_id()
        .times(1)
        .returning(move |_| Ok(stat.clone()));

    mock_repo
        .expect_find_log_values_in_range()
        .times(1)
        .returning(|_, _| {
            Ok(vec![
                DatedValue::new(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(), json!(100)),
                DatedValue::new(NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(), json!(66)),
            ])
        });

    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/statistics/1/integer_amount/total?start_date=2024-01-01&end_date=2024-01-02")
        .await;

    assert_eq!(response.status_code(), 200);
    let body: serde_json::Value = response.json();
    assert_eq!(body["total"], 166);
}

#[tokio::test]
async fn test_get_integer_amount_total_wrong_stat_type_returns_400() {
    let mut mock_repo = MockStatisticRepository::new();
    let stat = AnyStatistic::Habit(HabitStatistic::new(
        StatisticMetadata::new(StatisticId(1), "Health".into(), "Walked".into()),
        None,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    ));

    mock_repo
        .expect_find_metadata_by_id()
        .times(1)
        .returning(move |_| Ok(stat.clone()));

    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/statistics/1/integer_amount/total?start_date=2024-01-01&end_date=2024-01-02")
        .await;

    assert_eq!(response.status_code(), 400);
}

#[tokio::test]
async fn test_get_integer_amount_total_stat_not_found_returns_404() {
    let mut mock_repo = MockStatisticRepository::new();
    mock_repo
        .expect_find_metadata_by_id()
        .times(1)
        .returning(|_| Err(RepositoryError::NotFound));

    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/statistics/999/integer_amount/total?start_date=2024-01-01&end_date=2024-01-02")
        .await;

    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn test_get_integer_amount_total_invalid_date_range_returns_400() {
    let mock_repo = MockStatisticRepository::new();
    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/statistics/1/integer_amount/total?start_date=2024-01-03&end_date=2024-01-02")
        .await;

    assert_eq!(response.status_code(), 400);
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

// ============================================================================
// GET /statistics/:id/streak Tests
// ============================================================================

#[tokio::test]
async fn test_get_streak_success() {
    let mut mock_repo = MockStatisticRepository::new();
    let today = Utc::now().date_naive();
    let yesterday = today.checked_sub_days(Days::new(1)).unwrap();
    let two_days_ago = today.checked_sub_days(Days::new(2)).unwrap();

    let metadata = StatisticMetadata::new(
        StatisticId(1),
        "Fitness".to_string(),
        "Push Ups".to_string(),
    );
    let stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
        metadata,
        Some(AmountGoal::new(10, true)),
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    ));

    mock_repo
        .expect_find_metadata_by_id()
        .times(1)
        .returning(move |_| Ok(stat.clone()));

    mock_repo
        .expect_find_log_values_in_range()
        .times(1)
        .returning(move |_, _| {
            Ok(vec![
                DatedValue::new(two_days_ago, json!(5)),
                DatedValue::new(yesterday, json!(11)),
                DatedValue::new(today, json!(10)),
            ])
        });

    mock_repo
        .expect_find_goals_in_range()
        .times(1)
        .returning(|_, _| Ok(vec![]));

    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/statistics/1/streak").await;

    assert_eq!(response.status_code(), 200);
    let body: serde_json::Value = response.json();
    assert_eq!(body["statistic_id"], 1);
    assert_eq!(body["current_streak_days"], 2);
}

#[tokio::test]
async fn test_get_streak_statistic_not_found() {
    let mut mock_repo = MockStatisticRepository::new();

    mock_repo
        .expect_find_metadata_by_id()
        .times(1)
        .returning(|_| Err(RepositoryError::NotFound));

    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/statistics/999/streak").await;

    assert_eq!(response.status_code(), 404);
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"], "The requested resource was not found.");
}

#[tokio::test]
async fn test_get_streak_repository_error() {
    let mut mock_repo = MockStatisticRepository::new();

    mock_repo
        .expect_find_metadata_by_id()
        .times(1)
        .returning(|_| {
            Err(RepositoryError::OperationFailed(anyhow::anyhow!(
                "Database offline"
            )))
        });

    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);
    let server = TestServer::new(app).unwrap();

    let response = server.get("/statistics/1/streak").await;

    assert_eq!(response.status_code(), 500);
    let body: serde_json::Value = response.json();
    assert_eq!(body["error"], "Internal server error");
}

// ============================================================================
// GET /statistics/:id/logged/:date Tests
// ============================================================================

#[tokio::test]
async fn test_get_logged_value_has_value() {
    use chrono::NaiveDate;
    use lifestats_core::shared::ids::StatisticId;
    use serde_json::json;

    // Create mock repository
    let mut mock_repo = MockStatisticRepository::new();

    let stat = mock_integer_statistic(1, "Push Ups", "Fitness");
    let date = NaiveDate::from_ymd_opt(2025, 11, 18).unwrap();

    mock_repo
        .expect_find_metadata_by_id()
        .times(1)
        .returning(move |_| Ok(stat.clone()));

    mock_repo
        .expect_find_log_value()
        .with(
            mockall::predicate::eq(StatisticId(1)),
            mockall::predicate::eq(date),
        )
        .times(1)
        .returning(move |_, _| Ok(Some(json!(42))));

    // Create app state and router
    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);

    // Create test server
    let server = TestServer::new(app).unwrap();

    // Send request
    let response = server.get("/statistics/1/logged/2025-11-18").await;

    // Assert status code
    assert_eq!(response.status_code(), 200);

    // Assert response body
    let body: serde_json::Value = response.json();
    assert_eq!(body["statistic_id"], 1);
    assert_eq!(body["date"], "2025-11-18");
    assert_eq!(body["has_value"], true);
    assert_eq!(body["value"], json!(42));
}

#[tokio::test]
async fn test_get_logged_value_no_value() {
    use chrono::NaiveDate;
    use lifestats_core::shared::ids::StatisticId;

    // Create mock repository
    let mut mock_repo = MockStatisticRepository::new();

    let stat = mock_integer_statistic(1, "Push Ups", "Fitness");
    let date = NaiveDate::from_ymd_opt(2025, 11, 18).unwrap();

    mock_repo
        .expect_find_metadata_by_id()
        .times(1)
        .returning(move |_| Ok(stat.clone()));

    mock_repo
        .expect_find_log_value()
        .with(
            mockall::predicate::eq(StatisticId(1)),
            mockall::predicate::eq(date),
        )
        .times(1)
        .returning(move |_, _| Ok(None));

    // Create app state and router
    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);

    // Create test server
    let server = TestServer::new(app).unwrap();

    // Send request
    let response = server.get("/statistics/1/logged/2025-11-18").await;

    // Assert status code
    assert_eq!(response.status_code(), 200);

    // Assert response body
    let body: serde_json::Value = response.json();
    assert_eq!(body["statistic_id"], 1);
    assert_eq!(body["date"], "2025-11-18");
    assert_eq!(body["has_value"], false);
    // value field should not be present when None
    assert!(body.get("value").is_none() || body["value"].is_null());
}

#[tokio::test]
async fn test_get_logged_value_statistic_not_found() {
    // Create mock repository
    let mut mock_repo = MockStatisticRepository::new();

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
    let response = server.get("/statistics/999/logged/2025-11-18").await;

    // Assert status code
    assert_eq!(response.status_code(), 404);

    // Assert error response
    let body: serde_json::Value = response.json();
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("Statistic 999 not found"));
}

#[tokio::test]
async fn test_get_logged_value_derived_statistic() {
    use chrono::NaiveDate;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::types::derived_statistics::summing_statistic::SummingStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

    // Create mock repository
    let mut mock_repo = MockStatisticRepository::new();

    // Create a derived statistic (SummingStatistic)
    let metadata =
        StatisticMetadata::new(StatisticId(1), "Test".to_string(), "Total Work".to_string());
    let derived_stat = AnyStatistic::Summing(SummingStatistic::new(
        metadata,
        vec![],
        None,
        NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    ));

    mock_repo
        .expect_find_metadata_by_id()
        .times(1)
        .returning(move |_| Ok(derived_stat.clone()));

    // Create app state and router
    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);

    // Create test server
    let server = TestServer::new(app).unwrap();

    // Send request
    let response = server.get("/statistics/1/logged/2025-11-18").await;

    // Assert status code
    assert_eq!(response.status_code(), 400);

    // Assert error response
    let body: serde_json::Value = response.json();
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("derived statistics"));
}

#[tokio::test]
async fn test_get_logged_value_invalid_date_format() {
    // Create mock repository (no expectations needed - validation fails at routing level)
    let mock_repo = MockStatisticRepository::new();

    // Create app state and router
    let app_state = mock_app_state(Arc::new(mock_repo));
    let app = create_router(app_state);

    // Create test server
    let server = TestServer::new(app).unwrap();

    // Send request with invalid date format
    let response = server.get("/statistics/1/logged/2025-13-45").await;

    // Assert status code (Axum returns 400 for invalid path parameter parsing)
    assert_eq!(response.status_code(), 400);
}

// ============================================================================
// GET /statistics/:id/goals/:date Tests
// ============================================================================

async fn daily_goal_response_for(result: GoalQueryResult) -> serde_json::Value {
    let mut mock_repo = MockStatisticRepository::new();
    mock_repo
        .expect_find_goal()
        .times(1)
        .return_once(move |_, _| Ok(result));

    let app = create_router(mock_app_state(Arc::new(mock_repo)));
    let server = TestServer::new(app).unwrap();

    let response = server.get("/statistics/1/goals/2025-01-01").await;
    assert_eq!(response.status_code(), 200);
    response.json()
}

#[tokio::test]
async fn test_get_daily_goal_not_set() {
    let body = daily_goal_response_for(GoalQueryResult::NotSet).await;
    assert_eq!(body, json!({"kind": "not_set"}));
}

#[tokio::test]
async fn test_get_daily_goal_suppressed() {
    let body = daily_goal_response_for(GoalQueryResult::Suppressed).await;
    assert_eq!(body, json!({"kind": "suppressed"}));
}

#[tokio::test]
async fn test_get_daily_goal_with_goal() {
    let goal = json!({"target_amount": 10, "is_at_least": true});
    let body = daily_goal_response_for(GoalQueryResult::Goal(goal.clone())).await;
    assert_eq!(body, json!({"kind": "goal", "goal_data": goal}));
}

// ============================================================================
// GET /statistics/:id/achievements Tests
// ============================================================================

#[tokio::test]
async fn test_get_daily_achievements_happy_path() {
    let mut mock_repo = MockStatisticRepository::new();

    let stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
        StatisticMetadata::new(StatisticId(1), "Fitness".into(), "Push Ups".into()),
        Some(AmountGoal::new(10, true)),
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    ));
    let day1 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let day2 = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
    let day3 = NaiveDate::from_ymd_opt(2025, 1, 3).unwrap();

    mock_repo
        .expect_find_metadata_by_id()
        .times(1)
        .returning(move |_| Ok(stat.clone()));
    mock_repo
        .expect_find_log_values_in_range()
        .times(1)
        .returning(move |_, _| {
            Ok(vec![
                DatedValue::new(day1, json!(10)),
                DatedValue::new(day2, json!(5)),
            ])
        });
    // Called once by the evaluator's GoalResolver and once by the query handler
    mock_repo
        .expect_find_goals_in_range()
        .times(2)
        .returning(move |_, _| Ok(vec![DatedValue::new(day3, GoalQueryResult::Suppressed)]));

    let app = create_router(mock_app_state(Arc::new(mock_repo)));
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/statistics/1/achievements?start=2025-01-01&end=2025-01-03")
        .await;
    assert_eq!(response.status_code(), 200);

    let body: serde_json::Value = response.json();
    assert_eq!(body["statistic_id"], 1);
    assert_eq!(body["start_date"], "2025-01-01");
    assert_eq!(body["end_date"], "2025-01-03");
    assert_eq!(
        body["days"],
        json!([
            {"date": "2025-01-01", "has_goal": true, "achieved": true, "percentage_achieved": 100, "override": "none"},
            {"date": "2025-01-02", "has_goal": true, "achieved": false, "percentage_achieved": 50, "override": "none"},
            {"date": "2025-01-03", "has_goal": false, "achieved": false, "percentage_achieved": null, "override": "suppressed"},
        ])
    );
}

#[tokio::test]
async fn test_get_daily_achievements_statistic_not_found() {
    let mut mock_repo = MockStatisticRepository::new();
    mock_repo
        .expect_find_metadata_by_id()
        .times(1)
        .returning(|_| Err(RepositoryError::NotFound));

    let app = create_router(mock_app_state(Arc::new(mock_repo)));
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/statistics/999/achievements?start=2025-01-01&end=2025-01-03")
        .await;
    assert_eq!(response.status_code(), 404);

    let body: serde_json::Value = response.json();
    assert_eq!(body["error"], "The requested resource was not found.");
}

#[tokio::test]
async fn test_get_daily_achievements_invalid_range() {
    let mock_repo = MockStatisticRepository::new();
    let app = create_router(mock_app_state(Arc::new(mock_repo)));
    let server = TestServer::new(app).unwrap();

    let response = server
        .get("/statistics/1/achievements?start=2025-01-05&end=2025-01-01")
        .await;
    assert_eq!(response.status_code(), 400);
}
