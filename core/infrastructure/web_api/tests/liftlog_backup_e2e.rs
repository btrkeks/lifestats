//! End-to-end test for the LiftLog remote-backup ingest route: a synthetic
//! gzipped SQLite snapshot goes in over HTTP, statistics come out of the
//! repository.

use axum_test::TestServer;
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sql_types::Text;
use diesel::SqliteConnection;
use diesel_migrations::MigrationHarness;
use flate2::write::GzEncoder;
use flate2::Compression;
use lifestats_application::use_cases::integration::{
    BundleFactory, IntegrationService, LiftLogBackupParser, LiftLogBundleFactory,
};
use lifestats_application::SessionManager;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_liftlog_backup::SqliteLiftLogBackupParser;
use lifestats_persistence::pool::MIGRATIONS;
use lifestats_persistence::progress_repository::SqliteProgressRepository;
use lifestats_persistence::repository::SqliteStatisticRepository;
use lifestats_plotting::PlottersRenderer;
use lifestats_web_api::routes::create_router;
use lifestats_web_api::state::AppState;
use std::io::Write;
use std::sync::Arc;

mod helpers;
use helpers::NoOpEventPublisher;

const SESSION_JSON: &str = r#"{
    "version": 4,
    "id": "e2e-session-1",
    "blueprint": { "name": "Push", "notes": "" },
    "date": "2026-08-12",
    "recordedExercises": [
        {
            "type": "RecordedWeightedExercise",
            "blueprint": { "usesBodyweight": false },
            "potentialSets": [
                {
                    "weight": { "unit": "kilograms", "value": "60" },
                    "set": { "repsCompleted": 10, "completionDateTime": "2026-08-12T18:00:00+02:00" }
                },
                {
                    "weight": { "unit": "kilograms", "value": "60" },
                    "set": { "repsCompleted": 8, "completionDateTime": "2026-08-12T18:25:00+02:00" }
                }
            ]
        }
    ]
}"#;

fn build_backup(payloads: &[(&str, &str)]) -> Vec<u8> {
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let path = temp_file.path().to_string_lossy().into_owned();
    {
        let mut connection = SqliteConnection::establish(&path).unwrap();
        diesel::sql_query(
            "CREATE TABLE session (id TEXT PRIMARY KEY NOT NULL, payload TEXT NOT NULL)",
        )
        .execute(&mut connection)
        .unwrap();
        for (id, payload) in payloads {
            diesel::sql_query("INSERT INTO session (id, payload) VALUES (?, ?)")
                .bind::<Text, _>(*id)
                .bind::<Text, _>(*payload)
                .execute(&mut connection)
                .unwrap();
        }
    }
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&std::fs::read(&path).unwrap()).unwrap();
    encoder.finish().unwrap()
}

#[tokio::test]
async fn test_liftlog_backup_ingest_e2e() {
    // 1. SETUP: in-memory database with real repositories and parser
    let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
    let pool = Pool::builder().max_size(1).build(manager).unwrap();
    {
        let mut conn = pool.get().unwrap();
        conn.run_pending_migrations(MIGRATIONS).unwrap();
    }

    let repo = Arc::new(SqliteStatisticRepository::new(Arc::new(pool.clone())));
    let progress_repo = Arc::new(SqliteProgressRepository::new(Arc::new(pool)));
    let factories: Vec<Arc<dyn BundleFactory>> = vec![Arc::new(LiftLogBundleFactory)];
    let parser: Arc<dyn LiftLogBackupParser> = Arc::new(SqliteLiftLogBackupParser);
    let integration_service = Arc::new(IntegrationService::new(
        repo.clone() as Arc<dyn StatisticRepository + Send + Sync>,
        factories,
        None,
        None,
        Some(parser),
    ));
    let session_manager = Arc::new(SessionManager::new(repo.clone()));

    let app_state = AppState::new(
        repo.clone(),
        progress_repo,
        Arc::new(NoOpEventPublisher),
        Arc::new(PlottersRenderer::new()),
        integration_service,
        session_manager,
    );
    let server = TestServer::new(create_router(app_state)).unwrap();

    // 2. ACT: upload a snapshot exactly like LiftLog's remote backup does
    let backup = build_backup(&[("e2e-session-1", SESSION_JSON)]);
    let response = server
        .post("/integrations/liftlog/backup")
        .content_type("application/octet-stream")
        .bytes(backup.clone().into())
        .await;

    // 3. ASSERT: response contract
    assert_eq!(response.status_code(), 200);
    let body: serde_json::Value = response.json();
    assert_eq!(body["workouts_imported"], 1);
    assert_eq!(body["days_logged"], 1);

    // The bundle was auto-provisioned; resolve the statistic IDs the same way
    // a client would.
    let provision_response = server
        .post("/integrations/liftlog")
        .json(&serde_json::json!({}))
        .await;
    assert_eq!(provision_response.status_code(), 200);
    let ids: serde_json::Value = provision_response.json();
    let workout_time_id = ids["workout_time_id"].as_u64().unwrap() as u32;
    let total_weight_id = ids["total_weight_id"].as_u64().unwrap() as u32;

    // 4. ASSERT: values landed in the repository
    let date = NaiveDate::from_ymd_opt(2026, 8, 12).unwrap();
    let frames = repo
        .find_log_value(
            lifestats_core::shared::ids::StatisticId(workout_time_id),
            date,
        )
        .unwrap()
        .expect("workout time frames should be logged");
    assert_eq!(
        frames,
        serde_json::json!([{"start": "18:00:00", "end": "18:25:00"}])
    );

    let weight = repo
        .find_log_value(
            lifestats_core::shared::ids::StatisticId(total_weight_id),
            date,
        )
        .unwrap()
        .expect("total weight should be logged");
    assert_eq!(weight, serde_json::json!(1080));

    // 5. ACT again with the identical snapshot: must be a no-op, not an error
    let response = server
        .post("/integrations/liftlog/backup")
        .content_type("application/octet-stream")
        .bytes(backup.into())
        .await;
    assert_eq!(response.status_code(), 200);

    // 6. ASSERT: rejects garbage cleanly
    let response = server
        .post("/integrations/liftlog/backup")
        .content_type("application/octet-stream")
        .bytes(b"definitely not gzip".to_vec().into())
        .await;
    assert_eq!(response.status_code(), 400);
}
