use axum_test::TestServer;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use diesel_migrations::MigrationHarness;
use lifestats_application::use_cases::integration::{
    BundleFactory, IntegrationService, YomikataBundleFactory,
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

mod helpers;
use helpers::NoOpEventPublisher;

fn server() -> TestServer {
    let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
    let pool = Pool::builder().max_size(1).build(manager).unwrap();
    pool.get()
        .unwrap()
        .run_pending_migrations(MIGRATIONS)
        .unwrap();

    let repo = Arc::new(SqliteStatisticRepository::new(Arc::new(pool.clone())));
    let progress_repo = Arc::new(SqliteProgressRepository::new(Arc::new(pool)));
    let factories: Vec<Arc<dyn BundleFactory>> = vec![Arc::new(YomikataBundleFactory)];
    let integration_service = Arc::new(IntegrationService::new(
        repo.clone() as Arc<dyn StatisticRepository + Send + Sync>,
        factories,
    ));
    let session_manager = Arc::new(SessionManager::new(repo.clone()));

    let app_state = AppState::new(
        repo,
        progress_repo,
        Arc::new(NoOpEventPublisher),
        Arc::new(PlottersRenderer::new()),
        integration_service,
        session_manager,
    );
    TestServer::new(create_router(app_state)).unwrap()
}

#[tokio::test]
async fn test_yomikata_bundle_provisions_once_with_cards_goal() {
    let server = server();

    let first = server.post("/integrations/yomikata").json(&json!({})).await;
    assert_eq!(first.status_code(), 200);
    let ids = json!({"cards_reviewed_id": 1, "study_minutes_id": 2});
    assert_eq!(first.json::<serde_json::Value>(), ids);

    let second = server.post("/integrations/yomikata").json(&json!({})).await;
    assert_eq!(second.status_code(), 200);
    assert_eq!(second.json::<serde_json::Value>(), ids);

    let statistics = server.get("/statistics").await;
    assert_eq!(statistics.status_code(), 200);
    let mut statistics: Vec<serde_json::Value> = statistics.json();
    statistics.sort_by_key(|s| s["id"].as_u64());
    assert_eq!(
        json!(statistics),
        json!([
            {
                "id": 1,
                "name": "[Yomikata] Cards Reviewed",
                "category": "Yomikata",
                "statistic_type": "IntegerAmount",
                "default_goal": {"is_at_least": true, "target_amount": 400},
            },
            {
                "id": 2,
                "name": "[Yomikata] Study Time (minutes)",
                "category": "Yomikata",
                "statistic_type": "IntegerAmount",
                "default_goal": null,
            },
        ])
    );
}
