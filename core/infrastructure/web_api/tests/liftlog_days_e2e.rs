//! End-to-end test for `PUT /integrations/liftlog/days` against an in-memory
//! SQLite database with the real repositories.

use axum_test::TestServer;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use diesel_migrations::MigrationHarness;
use lifestats_application::use_cases::integration::{
    BundleFactory, IntegrationService, LiftLogBundleFactory,
};
use lifestats_application::SessionManager;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_persistence::pool::MIGRATIONS;
use lifestats_persistence::progress_repository::SqliteProgressRepository;
use lifestats_persistence::repository::SqliteStatisticRepository;
use lifestats_plotting::PlottersRenderer;
use lifestats_web_api::routes::create_router;
use lifestats_web_api::state::AppState;
use serde_json::{json, Value};
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
    let factories: Vec<Arc<dyn BundleFactory>> = vec![Arc::new(LiftLogBundleFactory)];
    let integration_service = Arc::new(IntegrationService::new(
        repo.clone() as Arc<dyn StatisticRepository + Send + Sync>,
        factories,
        None,
        None,
        None,
    ));
    let app_state = AppState::new(
        repo.clone(),
        Arc::new(SqliteProgressRepository::new(Arc::new(pool))),
        Arc::new(NoOpEventPublisher),
        Arc::new(PlottersRenderer::new()),
        integration_service,
        Arc::new(SessionManager::new(repo)),
    );
    TestServer::new(create_router(app_state)).unwrap()
}

fn workout(id: &str, start: &str, end: &str) -> Value {
    json!({ "id": id, "start": start, "end": end })
}

async fn put_days(server: &TestServer, days: Value) -> (u16, Value) {
    let response = server
        .put("/integrations/liftlog/days")
        .json(&json!({ "days": days }))
        .await;
    (response.status_code().as_u16(), response.json())
}

async fn logged(server: &TestServer, id: u64, date: &str) -> Value {
    server
        .get(&format!("/statistics/{id}/logged/{date}"))
        .await
        .json()
}

#[tokio::test]
async fn liftlog_days_replace_exactly_the_named_days() {
    let server = server();
    let gym_day = json!({
        "date": "2026-08-12",
        "workouts": [
            workout("a", "2026-08-12T18:00:00+02:00", "2026-08-12T18:30:00+02:00"),
            workout("b", "2026-08-12T18:20:00+02:00", "2026-08-12T18:55:00+02:00"),
        ]
    });

    let (status, body) = put_days(
        &server,
        json!([
            gym_day,
            { "date": "2026-08-13", "workouts": [workout("c", "2026-08-13T07:00:00+02:00", "2026-08-13T07:30:00+02:00")] },
            { "date": "2026-08-14", "workouts": [workout("d", "2026-08-14T09:15:00+02:00", "2026-08-14T09:15:00+02:00")] },
        ]),
    )
    .await;
    assert_eq!(status, 200);
    let id = body["statistic_id"].as_u64().unwrap();
    assert_eq!(
        body,
        json!({ "statistic_id": id, "days_updated": 3, "days_cleared": 0, "days_unchanged": 0 })
    );
    assert_eq!(
        logged(&server, id, "2026-08-12").await,
        json!({
            "statistic_id": id,
            "date": "2026-08-12",
            "has_value": true,
            "value": [{ "start": "18:00:00", "end": "18:55:00" }]
        })
    );

    // Clearing 08-13 leaves 08-12 (resent unchanged) and 08-14 (not named) alone.
    let (status, body) = put_days(
        &server,
        json!([gym_day, { "date": "2026-08-13", "workouts": [] }]),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(
        body,
        json!({ "statistic_id": id, "days_updated": 0, "days_cleared": 1, "days_unchanged": 1 })
    );
    assert_eq!(
        logged(&server, id, "2026-08-13").await,
        json!({ "statistic_id": id, "date": "2026-08-13", "has_value": false })
    );
    assert_eq!(
        logged(&server, id, "2026-08-14").await["value"],
        json!([{ "start": "09:15:00", "end": "09:16:00" }])
    );

    let (status, body) = put_days(
        &server,
        json!([gym_day, { "date": "2026-08-13", "workouts": [] }]),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(
        body,
        json!({ "statistic_id": id, "days_updated": 0, "days_cleared": 0, "days_unchanged": 2 })
    );
}

#[tokio::test]
async fn liftlog_days_rejects_invalid_requests() {
    let server = server();
    let late = workout(
        "late",
        "2026-08-12T19:00:00+02:00",
        "2026-08-12T18:00:00+02:00",
    );
    let naive = workout("naive", "2026-08-12T18:00:00", "2026-08-12T18:30:00+02:00");
    let ok = workout(
        "ok",
        "2026-08-12T18:00:00+02:00",
        "2026-08-12T18:30:00+02:00",
    );
    let too_many_days: Vec<Value> = (0..501)
        .map(|i| {
            let date = chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap() + chrono::Days::new(i);
            json!({ "date": date.to_string(), "workouts": [] })
        })
        .collect();

    let cases = [
        (json!([]), "days must not be empty"),
        (
            json!(too_many_days),
            "at most 500 days per request, got 501",
        ),
        (
            json!([
                { "date": "2026-08-12", "workouts": [] },
                { "date": "2026-08-12", "workouts": [ok] },
            ]),
            "duplicate date 2026-08-12",
        ),
        (
            json!([{ "date": "2026-08-12", "workouts": vec![ok.clone(); 101] }]),
            "date 2026-08-12 has 101 workouts, at most 100 allowed",
        ),
        (
            json!([{ "date": "2026-08-12", "workouts": [naive] }]),
            "workout naive: start is not an RFC 3339 timestamp: 2026-08-12T18:00:00",
        ),
        (
            json!([{ "date": "2026-08-12", "workouts": [late] }]),
            "workout late: end is before start",
        ),
    ];
    for (days, error) in cases {
        assert_eq!(
            put_days(&server, days).await,
            (400, json!({ "error": error }))
        );
    }

    let response = server
        .put("/integrations/liftlog/days")
        .content_type("application/json")
        .text("{\"days\": [")
        .await;
    assert_eq!(response.status_code(), 400);
    assert!(response.json::<Value>()["error"].is_string());
}
