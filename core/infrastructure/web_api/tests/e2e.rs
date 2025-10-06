use anyhow::Result;
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::RunQueryDsl;
use diesel::SqliteConnection;
use diesel_migrations::MigrationHarness;
use lifestats_application::use_cases::{
    goal_management_service::GoalManagementService,
    integration::{AnkiBundleFactory, BundleFactory, IntegrationService},
    logging_service::LoggingService,
    plotting_service::PlottingService,
    statistic_query_service::StatisticQueryService,
};
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
use lifestats_persistence::pool::MIGRATIONS;
use lifestats_persistence::repository::SqliteStatisticRepository;
use lifestats_persistence::SqlitePlotRepository;
use lifestats_plotting::PlottersRenderer;
use lifestats_web_api::routes::create_router;
use lifestats_web_api::state::AppState;
use std::sync::Arc;
use tokio::net::TcpListener;

mod helpers;
use helpers::NoOpEventPublisher;

/// Full end-to-end test: POST to log a value, then verify it was saved in the database
#[tokio::test]
async fn test_log_and_get_e2e() -> Result<()> {
    // ========================================================================
    // 1. SETUP: Create in-memory database and run migrations
    // ========================================================================

    let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
    let pool = Pool::builder()
        .max_size(1)
        .build(manager)
        .expect("Failed to create test pool");

    // Run migrations
    {
        let mut conn = pool.get().expect("Failed to get connection for migrations");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");
    }

    // ========================================================================
    // 2. SEED: Insert a known statistic
    // ========================================================================

    let test_statistic_id = 1u32;
    let metadata = StatisticMetadata::new(
        StatisticId(test_statistic_id),
        "Fitness".to_string().into_boxed_str(),
        "Push Ups".to_string().into_boxed_str(),
    );
    let test_stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
        metadata,
        None,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    ));

    // Save the statistic to the database
    let repo = Arc::new(SqliteStatisticRepository::new(Arc::new(pool.clone())));
    repo.save_metadata(&test_stat)?;

    // ========================================================================
    // 3. BUILD: Create real AppState with real services
    // ========================================================================

    let publisher = Arc::new(NoOpEventPublisher);
    let plot_repo = Arc::new(SqlitePlotRepository::new(Arc::new(pool.clone())));
    let renderer = Arc::new(PlottersRenderer::new());

    let logging_service = Arc::new(LoggingService::new(
        Arc::clone(&repo),
        Arc::clone(&publisher),
    ));
    let goal_service = Arc::new(GoalManagementService::new(
        Arc::clone(&repo),
        Arc::clone(&publisher),
    ));
    let query_service = Arc::new(StatisticQueryService::new(Arc::clone(&repo)));
    let plotting_service = Arc::new(PlottingService::new(Arc::clone(&repo), plot_repo, renderer));

    // Create integration service
    let factories: Vec<Arc<dyn BundleFactory>> = vec![Arc::new(AnkiBundleFactory)];
    let integration_service = Arc::new(IntegrationService::new(
        repo.clone() as Arc<dyn StatisticRepository + Send + Sync>,
        factories,
    ));

    let app_state = AppState::new(
        logging_service,
        goal_service,
        query_service,
        plotting_service,
        integration_service,
    );

    // ========================================================================
    // 4. START SERVER: Spawn axum server on a random free port
    // ========================================================================

    let app = create_router(app_state);
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;

    // Spawn server in background
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give server a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // ========================================================================
    // 5. ACT & ASSERT (POST): Send a POST request to log a value
    // ========================================================================

    let client = reqwest::Client::new();
    let url = format!(
        "http://127.0.0.1:{}/statistics/{}/log/2024-10-10",
        addr.port(),
        test_statistic_id
    );

    let payload = serde_json::json!({
        "value": "25"
    });

    let response = client.post(&url).json(&payload).send().await?;

    assert_eq!(
        response.status(),
        204,
        "Expected 204 No Content, got {}",
        response.status()
    );

    // ========================================================================
    // 6. VERIFY: Check the database to confirm the value was saved
    // ========================================================================

    use lifestats_persistence::schema::logging_data;

    let mut conn = pool.get()?;
    let results: Vec<(i32, String, String)> = logging_data::table
        .select((
            logging_data::statistic_id,
            logging_data::date,
            logging_data::value,
        ))
        .filter(logging_data::statistic_id.eq(test_statistic_id as i32))
        .filter(logging_data::date.eq("2024-10-10"))
        .load(&mut conn)?;

    assert_eq!(results.len(), 1, "Expected 1 row in database");

    let (_, saved_date, saved_value_json) = &results[0];
    assert_eq!(saved_date, "2024-10-10");

    // The value should be saved as JSON
    let saved_value: serde_json::Value = serde_json::from_str(saved_value_json)?;
    assert_eq!(saved_value, 25);

    Ok(())
}
