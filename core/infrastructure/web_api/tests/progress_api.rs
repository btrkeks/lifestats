mod helpers;

use axum_test::TestServer;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use diesel_migrations::MigrationHarness;
use helpers::NoOpEventPublisher;
use lifestats_application::use_cases::integration::{
    AnkiBundleFactory, BundleFactory, EbookBundleFactory, IntegrationService,
};
use lifestats_application::SessionManager;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_persistence::pool::MIGRATIONS;
use lifestats_persistence::progress_repository::SqliteProgressRepository;
use lifestats_persistence::repository::SqliteStatisticRepository;
use lifestats_plotting::PlottersRenderer;
use lifestats_web_api::routes::create_router;
use lifestats_web_api::state::AppState;
use serde_json::json;
use std::sync::Arc;

fn progress_server() -> TestServer {
    let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
    let pool = Pool::builder()
        .max_size(1)
        .build(manager)
        .expect("failed to create test pool");

    {
        let mut conn = pool.get().expect("failed to get connection for migrations");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("failed to run migrations");
    }

    let pool = Arc::new(pool);
    let statistic_repo = Arc::new(SqliteStatisticRepository::new(pool.clone()));
    let progress_repo = Arc::new(SqliteProgressRepository::new(pool));
    let publisher = Arc::new(NoOpEventPublisher);
    let renderer = Arc::new(PlottersRenderer::new());
    let factories: Vec<Arc<dyn BundleFactory>> =
        vec![Arc::new(AnkiBundleFactory), Arc::new(EbookBundleFactory)];
    let integration_service = Arc::new(IntegrationService::new(
        statistic_repo.clone() as Arc<dyn StatisticRepository + Send + Sync>,
        factories,
        None,
    ));
    let session_manager = Arc::new(SessionManager::new(statistic_repo.clone()));

    let app_state = AppState::new(
        statistic_repo,
        progress_repo,
        publisher,
        renderer,
        integration_service,
        session_manager,
    );

    TestServer::new(create_router(app_state)).unwrap()
}

async fn create_book(server: &TestServer) -> u64 {
    let response = server
        .post("/progress-items")
        .json(&json!({
            "name": "The Left Hand of Darkness",
            "category": "Reading",
            "target_value": 100,
            "unit": { "type": "Pages" },
            "tags": { "media_type": "book" }
        }))
        .await;

    assert_eq!(response.status_code(), 201);
    let body: serde_json::Value = response.json();
    assert_eq!(body["name"], "The Left Hand of Darkness");
    body["id"].as_u64().unwrap()
}

#[tokio::test]
async fn progress_item_manual_flow_returns_summary_and_daily_deltas() {
    let server = progress_server();
    let item_id = create_book(&server).await;

    for (day, position) in [(1, 40), (2, 30), (3, 60)] {
        let response = server
            .post(&format!("/progress-items/{item_id}/checkpoints"))
            .json(&json!({
                "recorded_at": format!("2026-05-{day:02}T20:00:00Z"),
                "effective_date": format!("2026-05-{day:02}"),
                "position": position,
                "source": null,
                "source_event_id": null,
                "note": null
            }))
            .await;
        assert_eq!(response.status_code(), 201);
    }

    let summary_response = server
        .get(&format!("/progress-items/{item_id}/summary"))
        .await;
    assert_eq!(summary_response.status_code(), 200);
    let summary: serde_json::Value = summary_response.json();
    assert_eq!(summary["latest_position"], 60);
    assert_eq!(summary["furthest_position"], 60);
    assert_eq!(summary["percent_furthest"], 0.6);

    let deltas_response = server
        .get(&format!(
            "/progress-items/{item_id}/daily-deltas?start=2026-05-01&end=2026-05-03"
        ))
        .await;
    assert_eq!(deltas_response.status_code(), 200);
    let deltas: serde_json::Value = deltas_response.json();
    assert_eq!(
        deltas,
        json!([
            { "date": "2026-05-01", "value": 40 },
            { "date": "2026-05-03", "value": 20 }
        ])
    );
}

#[tokio::test]
async fn progress_checkpoint_source_event_is_idempotent() {
    let server = progress_server();
    let item_id = create_book(&server).await;
    let payload = json!({
        "recorded_at": "2026-05-02T20:00:00Z",
        "effective_date": "2026-05-02",
        "position": 42,
        "source": "ebook",
        "source_event_id": "event-1",
        "note": null
    });

    let first = server
        .post(&format!("/progress-items/{item_id}/checkpoints"))
        .json(&payload)
        .await;
    assert_eq!(first.status_code(), 201);
    let first_body: serde_json::Value = first.json();
    assert_eq!(first_body["status"], "inserted");

    let second = server
        .post(&format!("/progress-items/{item_id}/checkpoints"))
        .json(&payload)
        .await;
    assert_eq!(second.status_code(), 200);
    let second_body: serde_json::Value = second.json();
    assert_eq!(second_body["status"], "duplicate");

    let checkpoints = server
        .get(&format!("/progress-items/{item_id}/checkpoints"))
        .await;
    let body: serde_json::Value = checkpoints.json();
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn progress_validation_rejects_position_above_target() {
    let server = progress_server();
    let item_id = create_book(&server).await;

    let response = server
        .post(&format!("/progress-items/{item_id}/checkpoints"))
        .json(&json!({
            "recorded_at": "2026-05-02T20:00:00Z",
            "effective_date": "2026-05-02",
            "position": 101
        }))
        .await;

    assert_eq!(response.status_code(), 400);
}
