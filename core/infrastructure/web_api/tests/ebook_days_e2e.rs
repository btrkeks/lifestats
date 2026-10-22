use axum::{http::StatusCode, Router};
use axum_test::TestServer;
use chrono::{Duration, Utc};
use diesel::{
    connection::SimpleConnection,
    prelude::*,
    r2d2::{ConnectionManager, CustomizeConnection, Pool},
    sql_types::Text,
};
use diesel_migrations::MigrationHarness;
use lifestats_application::{
    use_cases::integration::{EbookBundleFactory, EbookProgressService, IntegrationService},
    SessionManager,
};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_persistence::{
    pool::MIGRATIONS, progress_repository::SqliteProgressRepository,
    repository::SqliteStatisticRepository, SqliteEbookEventStore,
};
use lifestats_plotting::PlottersRenderer;
use lifestats_web_api::{routes::create_router, state::AppState};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Barrier},
};
mod helpers;
use helpers::NoOpEventPublisher;

const P1: &str = "0b6c3f0e-2a71-4d38-9f43-5a1e8c2d9b77";
const P2: &str = "0b6c3f0e-2a71-4d38-9f43-5a1e8c2d9b78";
const MON: &str = "2026-10-19";
const TUE: &str = "2026-10-20";
fn key(c: char) -> String {
    format!("ttu-text-v1:{}", c.to_string().repeat(64))
}
fn path(producer: &str, book: &str, day: &str) -> String {
    format!("/integrations/ebook/producers/{producer}/books/{book}/days/{day}")
}
fn body(spans: Value) -> Value {
    json!({"title":"Book","length":10000,"spans":spans,"last_position":5000,"observed_at":"2026-10-20T12:00:00Z"})
}
#[derive(Debug)]
struct Customize;
impl CustomizeConnection<SqliteConnection, diesel::r2d2::Error> for Customize {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        conn.batch_execute(
            "PRAGMA foreign_keys=ON; PRAGMA busy_timeout=30000; PRAGMA journal_mode=WAL;",
        )
        .map_err(diesel::r2d2::Error::QueryError)
    }
}
struct Fixture {
    server: TestServer,
    repo: Arc<SqliteStatisticRepository>,
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    db: PathBuf,
}
fn router(pool: Arc<Pool<ConnectionManager<SqliteConnection>>>) -> Router {
    let repo = Arc::new(SqliteStatisticRepository::new(pool.clone()));
    let progress = Arc::new(SqliteProgressRepository::new(pool.clone()));
    let service = Arc::new(
        IntegrationService::new(
            repo.clone(),
            vec![Arc::new(EbookBundleFactory)],
            Some(Arc::new(SqliteEbookEventStore::new(pool.clone()))),
            Some(Arc::new(EbookProgressService::new(progress.clone()))),
        )
        .with_ebook_reading_store(repo.clone()),
    );
    create_router(AppState::new(
        repo.clone(),
        progress,
        Arc::new(NoOpEventPublisher),
        Arc::new(PlottersRenderer::new()),
        service,
        Arc::new(SessionManager::new(repo)),
    ))
}
fn fixture() -> Fixture {
    let db = std::env::temp_dir().join(format!("lifestats-ebook-{}.db", uuid::Uuid::new_v4()));
    fixture_at(db)
}
fn fixture_at(db: PathBuf) -> Fixture {
    let pool = Arc::new(
        Pool::builder()
            .max_size(6)
            .connection_customizer(Box::new(Customize))
            .build(ConnectionManager::<SqliteConnection>::new(
                db.to_string_lossy(),
            ))
            .unwrap(),
    );
    pool.get()
        .unwrap()
        .run_pending_migrations(MIGRATIONS)
        .unwrap();
    let repo = Arc::new(SqliteStatisticRepository::new(pool.clone()));
    let server = TestServer::builder()
        .http_transport()
        .build(router(pool.clone()))
        .unwrap();
    Fixture {
        server,
        repo,
        pool,
        db,
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.db.display()));
        }
    }
}
fn statistic(f: &Fixture) -> u32 {
    let stats = f
        .repo
        .find_by_tags(&HashMap::from([
            ("source".into(), "ebook".into()),
            ("metric".into(), "characters_read".into()),
        ]))
        .unwrap();
    assert_eq!(stats.len(), 1);
    stats[0].get_metadata().id().0
}
async fn amount(f: &Fixture, day: &str) -> i64 {
    let r = f
        .server
        .get(&format!("/statistics/{}/logged/{day}", statistic(f)))
        .await;
    r.assert_status_ok();
    r.json::<Value>()["value"].as_i64().unwrap_or(0)
}
async fn put(f: &Fixture, p: &str, day: &str, b: &Value) {
    f.server
        .put(&path(p, &key('a'), day))
        .json(b)
        .await
        .assert_status(StatusCode::NO_CONTENT);
}
#[derive(QueryableByName)]
struct JsonRow {
    #[diesel(sql_type=Text)]
    value: String,
}
fn query(f: &Fixture, sql: &str) -> Value {
    let row = diesel::sql_query(sql)
        .get_result::<JsonRow>(&mut f.pool.get().unwrap())
        .unwrap();
    serde_json::from_str(&row.value).unwrap()
}
fn snapshot(f: &Fixture) -> Value {
    json!({
        "books":query(f,"SELECT json_group_array(json_array(book_key,length,progress_item_id,legacy_spans,created_at)) AS value FROM ebook_books"),
        "spans":query(f,"SELECT json_group_array(json_array(producer_id,book_key,day,spans,last_position,observed_at)) AS value FROM ebook_day_spans"),
        "credits":query(f,"SELECT json_group_array(json_array(book_key,day,characters)) AS value FROM ebook_day_credits"),
        "statistics":query(f,"SELECT json_group_array(json_array(id,name,tags)) AS value FROM statistics"),
        "values":query(f,"SELECT json_group_array(json_array(statistic_id,date,value)) AS value FROM logging_data"),
        "bindings":query(f,"SELECT json_group_array(json_array(metric,statistic_id)) AS value FROM ebook_metric_bindings"),
        "items":query(f,"SELECT json_group_array(json_array(id,name,target_value,tags,canonical_tags,created_at,updated_at)) AS value FROM progress_items"),
        "checkpoints":query(f,"SELECT json_group_array(json_array(id,progress_item_id,recorded_at,effective_date,position_value,source,source_event_id,created_at)) AS value FROM progress_checkpoints"),
        "claims":query(f,"SELECT json_group_array(json_array(reader_data_id,book_key)) AS value FROM ebook_legacy_bindings")
    })
}

#[tokio::test]
async fn replay_and_stale_subset_change_nothing() {
    let f = fixture();
    let report = body(json!([[0, 5000]]));
    put(&f, P1, TUE, &report).await;
    let original = snapshot(&f);
    put(&f, P1, TUE, &report).await;
    assert_eq!(snapshot(&f), original);
    let mut stale = body(json!([[0, 3000]]));
    stale["observed_at"] = json!("2026-10-19T12:00:00Z");
    stale["last_position"] = json!(3000);
    put(&f, P1, TUE, &stale).await;
    assert_eq!(snapshot(&f), original);
    assert_eq!(amount(&f, TUE).await, 5000);
    let item: Value = f.server.get("/progress-items/1").await.json();
    assert_eq!(item["target_value"], 10000);
}

#[tokio::test]
async fn two_producers_and_late_earlier_day_preserve_daily_residuals() {
    let f = fixture();
    put(&f, P1, TUE, &body(json!([[0, 3000]]))).await;
    f.server
        .post(&format!("/statistics/{}/log/{MON}", statistic(&f)))
        .json(&json!({"value":"17","update_policy":"sum"}))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    f.server
        .post(&format!("/statistics/{}/log/{TUE}", statistic(&f)))
        .json(&json!({"value":"23","update_policy":"sum"}))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    put(&f, P2, TUE, &body(json!([[0, 3000]]))).await;
    assert_eq!(amount(&f, TUE).await, 3023);
    put(&f, P2, MON, &body(json!([[0, 5000]]))).await;
    assert_eq!(amount(&f, MON).await, 5017);
    assert_eq!(amount(&f, TUE).await, 23);
    let credits = query(
        &f,
        "SELECT json_group_object(day,characters) AS value FROM ebook_day_credits",
    );
    assert_eq!(credits, json!({MON:5000,TUE:0}));
    for (day, residual) in [(MON, 17), (TUE, 23)] {
        assert_eq!(
            amount(&f, day).await,
            residual + credits[day].as_i64().unwrap()
        );
    }
    let mut other = body(json!([[0, 80]]));
    other["title"] = json!("Another book");
    f.server
        .put(&path(P1, &key('b'), MON))
        .json(&other)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert_eq!(amount(&f, MON).await, 5097);
    let totals=query(&f,"SELECT json_group_object(day,characters) AS value FROM (SELECT day,SUM(characters) AS characters FROM ebook_day_credits GROUP BY day)");
    for (day, residual) in [(MON, 17), (TUE, 23)] {
        assert_eq!(
            amount(&f, day).await,
            residual + totals[day].as_i64().unwrap()
        );
    }
}

#[tokio::test]
async fn newer_observation_updates_position_and_latest_uses_client_time() {
    let f = fixture();
    let mut later = body(json!([[0, 100]]));
    later["last_position"] = json!(100);
    later["observed_at"] = json!("2026-10-20T12:00:00.000001+00:00");
    put(&f, P1, TUE, &later).await;
    let mut earlier = body(json!([[0, 5000]]));
    earlier["last_position"] = json!(5000);
    earlier["observed_at"] = json!("2026-10-19T12:00:00Z");
    put(&f, P2, MON, &earlier).await;
    let summary: Value = f.server.get("/progress-items/1/summary").await.json();
    assert_eq!(summary["latest_position"], 100);
    assert_eq!(summary["furthest_position"], 5000);
    later["last_position"] = json!(50);
    later["observed_at"] = json!("2026-10-21T12:00:00Z");
    put(&f, P1, TUE, &later).await;
    let summary: Value = f.server.get("/progress-items/1/summary").await.json();
    assert_eq!(summary["latest_position"], 50);
    assert_eq!(amount(&f, MON).await, 5000);
    assert_eq!(amount(&f, TUE).await, 0);
}

#[tokio::test]
async fn injected_failure_rolls_back_every_projection_and_provisioning() {
    let f = fixture();
    let before = snapshot(&f);
    f.pool.get().unwrap().batch_execute("CREATE TRIGGER fail_ebook BEFORE INSERT ON progress_checkpoints BEGIN SELECT RAISE(ABORT,'injected after projections'); END;").unwrap();
    let r = f
        .server
        .put(&path(P1, &key('a'), MON))
        .json(&body(json!([[0, 100]])))
        .await;
    r.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(r.json::<Value>()["error"]["code"], "storage_error");
    assert_eq!(snapshot(&f), before);
    f.pool
        .get()
        .unwrap()
        .batch_execute("DROP TRIGGER fail_ebook;")
        .unwrap();
    put(&f, P1, MON, &body(json!([[0, 100]]))).await;
    let before = snapshot(&f);
    f.pool.get().unwrap().batch_execute("CREATE TRIGGER fail_ebook BEFORE INSERT ON progress_checkpoints BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    f.server
        .put(&path(P1, &key('a'), MON))
        .json(&body(json!([[0, 300]])))
        .await
        .assert_status(StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(snapshot(&f), before);
}

#[tokio::test]
async fn length_conflict_and_all_validation_errors_leave_state_untouched() {
    let f = fixture();
    let before = snapshot(&f);
    for (field, value) in [
        ("spans", json!([[100, 200], [0, 50]])),
        ("spans", json!([[0, 100], [50, 200]])),
        ("spans", json!([[0, 100], [100, 200]])),
        ("spans", json!([[0, 10001]])),
        ("spans", json!([[-1, 10]])),
        ("spans", json!([[0, 0]])),
        ("spans", json!([[0, 1.0]])),
        ("spans", json!([[0, 1, 2]])),
        ("spans", json!(vec![[0, 1]; 4097])),
        ("length", json!(0)),
        ("length", json!(1.0)),
        ("length", json!(9007199254740993u64)),
        ("last_position", json!(10001)),
        ("title", json!(" ")),
        ("title", json!("x".repeat(1001))),
        ("observed_at", json!("2026-10-20")),
        ("observed_at", json!(null)),
    ] {
        let mut b = body(json!([]));
        b[field] = value;
        let r = f.server.put(&path(P1, &key('a'), MON)).json(&b).await;
        r.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(r.json::<Value>()["error"]["field"], field);
        assert_eq!(snapshot(&f), before);
    }
    for (p, k, d, field) in [
        ("bad", key('a'), MON.to_owned(), "producer"),
        (P1, "a".repeat(64), MON.to_owned(), "book_key"),
        (P1, key('A'), MON.to_owned(), "book_key"),
        (P1, key('a'), "2026-02-29".into(), "day"),
        (P1, key('a'), "2026-1-01".into(), "day"),
        (
            P1,
            key('a'),
            (chrono::Local::now().date_naive() + Duration::days(3)).to_string(),
            "day",
        ),
    ] {
        let r = f.server.put(&path(p, &k, &d)).json(&body(json!([]))).await;
        r.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(r.json::<Value>()["error"]["field"], field);
        assert_eq!(snapshot(&f), before);
    }
    let mut extra = body(json!([]));
    extra["extra"] = json!(1);
    f.server
        .put(&path(P1, &key('a'), MON))
        .json(&extra)
        .await
        .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    let duplicate =
        serde_json::to_string(&body(json!([])))
            .unwrap()
            .replacen("{", "{\"length\":10000,", 1);
    f.server
        .put(&path(P1, &key('a'), MON))
        .text(&duplicate)
        .content_type("application/json")
        .await
        .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    for (raw, status, code) in [
        ("{", StatusCode::BAD_REQUEST, "malformed_json"),
        ("[]", StatusCode::UNPROCESSABLE_ENTITY, "invalid_field"),
        ("null", StatusCode::UNPROCESSABLE_ENTITY, "invalid_field"),
    ] {
        let r = f
            .server
            .put(&path(P1, &key('a'), MON))
            .text(raw)
            .content_type("application/json")
            .await;
        r.assert_status(status);
        assert_eq!(r.json::<Value>()["error"]["code"], code);
    }
    let r = f
        .server
        .put(&path(P1, &key('a'), MON))
        .text("{}")
        .content_type("text/plain")
        .await;
    r.assert_status(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(r.json::<Value>()["error"]["code"], "unsupported_media_type");
    assert_eq!(snapshot(&f), before);
    put(&f, P1, MON, &body(json!([]))).await;
    let before = snapshot(&f);
    let mut changed = body(json!([]));
    changed["length"] = json!(9999);
    let r = f.server.put(&path(P2, &key('a'), MON)).json(&changed).await;
    r.assert_status(StatusCode::CONFLICT);
    assert_eq!(r.json::<Value>()["error"]["code"], "book_length_mismatch");
    assert_eq!(snapshot(&f), before);
}

#[tokio::test]
async fn overflow_is_422_and_recreation_projects_final_all_book_credits_once() {
    let f = fixture();
    put(&f, P1, MON, &body(json!([[0, 100]]))).await;
    put(&f, P1, TUE, &body(json!([[50, 200]]))).await;
    let id = statistic(&f);
    f.server
        .post(&format!("/statistics/{id}/log/{MON}"))
        .json(&json!({"value":i64::MAX.to_string(),"update_policy":"overwrite"}))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    let before = snapshot(&f);
    let r = f
        .server
        .put(&path(P1, &key('a'), MON))
        .json(&body(json!([[0, 101]])))
        .await;
    r.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.json::<Value>()["error"]["code"], "projection_overflow");
    assert_eq!(snapshot(&f), before);
    let metadata = f
        .repo
        .find_metadata_by_id(lifestats_core::shared::ids::StatisticId(id))
        .unwrap();
    f.server
        .delete(&format!("/statistics/{id}"))
        .await
        .assert_status_success();
    let replacement = f
        .repo
        .create_metadata(metadata)
        .unwrap()
        .get_metadata()
        .id();
    f.repo
        .save_log_value(replacement, MON.parse().unwrap(), json!(7))
        .unwrap();
    put(&f, P2, MON, &body(json!([[0, 150]]))).await;
    assert_eq!(amount(&f, MON).await, 157);
    assert_eq!(amount(&f, TUE).await, 50);
    let before = snapshot(&f);
    put(&f, P2, MON, &body(json!([[0, 150]]))).await;
    assert_eq!(snapshot(&f), before);
}

#[tokio::test]
async fn descriptor_survives_reconstructed_router_and_database_identity_differs() {
    let f = fixture();
    let first: Value = f.server.get("/integrations/ebook").await.json();
    assert_eq!(first["protocol"], 1);
    uuid::Uuid::parse_str(first["server_id"].as_str().unwrap()).unwrap();
    let server = TestServer::builder()
        .http_transport()
        .build(router(f.pool.clone()))
        .unwrap();
    assert_eq!(
        server.get("/integrations/ebook").await.json::<Value>(),
        first
    );
    let other = fixture();
    assert_ne!(
        other
            .server
            .get("/integrations/ebook")
            .await
            .json::<Value>()["server_id"],
        first["server_id"]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_four_threads_two_producers_equal_sequential_application() {
    let concurrent = fixture();
    let sequential = fixture();
    let base = concurrent.server.server_address().unwrap();
    let barrier = Arc::new(Barrier::new(4));
    let reports = |thread: usize| -> Vec<(String, String, Value)> {
        (0..16)
            .flat_map(|i| {
                (0..2).map(move |p| {
                    let n = thread * 32 + i * 2 + p;
                    let start = (n * 31) % 1500;
                    let end = start + 500;
                    let mut report = body(json!([[start, end]]));
                    report["last_position"] = json!(end);
                    report["observed_at"] = json!(("2026-10-20T12:00:00Z"
                        .parse::<chrono::DateTime<Utc>>()
                        .unwrap()
                        + Duration::seconds(n as i64))
                    .to_rfc3339());
                    (
                        if p == 0 { P1.into() } else { P2.into() },
                        if i % 2 == 0 { MON.into() } else { TUE.into() },
                        report,
                    )
                })
            })
            .collect()
    };
    let mut threads = vec![];
    for t in 0..4 {
        let barrier = barrier.clone();
        let base = base.clone();
        let reports = reports(t);
        threads.push(std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            barrier.wait();
            runtime.block_on(async {
                let client = reqwest::Client::new();
                for (p, d, b) in reports {
                    let r = client
                        .put(base.join(&path(&p, &key('a'), &d)).unwrap())
                        .json(&b)
                        .send()
                        .await
                        .unwrap();
                    assert_eq!(
                        r.status(),
                        reqwest::StatusCode::NO_CONTENT,
                        "{}",
                        r.text().await.unwrap()
                    );
                }
            });
        }));
    }
    for thread in threads {
        thread.join().unwrap();
    }
    for t in 0..4 {
        for (p, d, b) in reports(t) {
            put(&sequential, &p, &d, &b).await;
        }
    }
    assert_eq!(amount(&concurrent, MON).await, 1988);
    assert_eq!(amount(&concurrent, TUE).await, 0);
    for day in [MON, TUE] {
        assert_eq!(
            amount(&concurrent, day).await,
            amount(&sequential, day).await
        );
    }
    for sql in ["SELECT json_group_array(json_array(producer_id,day,spans,last_position,observed_at)) AS value FROM (SELECT * FROM ebook_day_spans ORDER BY producer_id,day)","SELECT json_group_array(json_array(day,characters)) AS value FROM (SELECT * FROM ebook_day_credits ORDER BY day)","SELECT json_group_array(json_array(source_event_id,recorded_at,position_value)) AS value FROM (SELECT * FROM progress_checkpoints ORDER BY source_event_id)"] {assert_eq!(query(&concurrent,sql),query(&sequential,sql));}
}

#[tokio::test]
async fn legacy_post_routes_remain_idempotent_and_gate_matching_titles() {
    let f = fixture();
    f.server
        .post("/integrations/ebook")
        .json(&json!({}))
        .await
        .assert_status_ok();
    let event = json!({"event_id":"v7-leftover-1","date":MON,"metric":"characters_read","delta":80,"book_id":1,"book_title":"Book","confirmed_position_after":100,"book_target_characters":10000});
    let first = f
        .server
        .post("/integrations/ebook/events")
        .json(&event)
        .await;
    first.assert_status_ok();
    assert_eq!(first.json::<Value>()["accepted"], true);
    let second = f
        .server
        .post("/integrations/ebook/events")
        .json(&event)
        .await;
    second.assert_status_ok();
    assert_eq!(second.json::<Value>()["duplicate"], true);
    assert_eq!(amount(&f, MON).await, 80);
    let before = snapshot(&f);
    let r = f
        .server
        .put(&path(P2, &key('a'), MON))
        .json(&body(json!([[0, 500]])))
        .await;
    r.assert_status(StatusCode::CONFLICT);
    assert_eq!(
        r.json::<Value>()["error"]["code"],
        "legacy_adoption_pending"
    );
    assert_eq!(snapshot(&f), before);
    let mut different = body(json!([[0, 50]]));
    different["title"] = json!("Phone book 1");
    f.server
        .put(&path(P2, &key('b'), MON))
        .json(&different)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    let legacy: Value = f.server.get("/progress-items/1").await.json();
    assert_eq!(legacy["name"], "Book");
    assert_eq!(legacy["tags"]["reader_data_id"], "1");
}

async fn legacy_event(f: &Fixture, event_id: &str, position: u64, delta: u64) {
    f.server.post("/integrations/ebook/events").json(&json!({"event_id":event_id,"date":MON,"metric":"characters_read","delta":delta,"book_id":1,"book_title":"Book","confirmed_position_after":position,"book_target_characters":10000})).await.assert_status_ok();
}
fn claim(book: &str, title: &str, length: u64) -> Value {
    json!({"book_key":book,"title":title,"length":length})
}

#[tokio::test]
async fn adoption_seeds_transaction_time_furthest_without_credit_and_unblocks_phone() {
    let f = fixture();
    legacy_event(&f, "queued", 100, 80).await;
    legacy_event(&f, "tail", 150, 50).await;
    legacy_event(&f, "late-checkpoint", 120, 10).await;
    assert_eq!(amount(&f, MON).await, 140);
    let before = snapshot(&f);
    let r = f
        .server
        .put(&path(P2, &key('a'), MON))
        .json(&body(json!([[0, 100]])))
        .await;
    r.assert_status(StatusCode::CONFLICT);
    assert_eq!(
        r.json::<Value>()["error"]["code"],
        "legacy_adoption_pending"
    );
    assert_eq!(snapshot(&f), before);
    let r = f
        .server
        .put("/integrations/ebook/legacy-books/1")
        .json(&claim(&key('a'), "Wrong title", 10000))
        .await;
    r.assert_status(StatusCode::CONFLICT);
    assert_eq!(
        r.json::<Value>()["error"]["code"],
        "legacy_binding_conflict"
    );
    assert_eq!(snapshot(&f), before);
    f.server
        .put("/integrations/ebook/legacy-books/1")
        .json(&claim(&key('a'), "Book", 10000))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert_eq!(amount(&f, MON).await, 140);
    assert_eq!(
        query(&f, "SELECT json(legacy_spans) AS value FROM ebook_books"),
        json!([[0, 150]])
    );
    let item: Value = f.server.get("/progress-items/1").await.json();
    assert_eq!(item["tags"]["book_key"], key('a'));
    assert_eq!(item["tags"]["reader_data_id"], "1");
    let before = snapshot(&f);
    f.server
        .put("/integrations/ebook/legacy-books/1")
        .json(&claim(&key('a'), "Book", 10000))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert_eq!(snapshot(&f), before);
    for (id, b, code) in [
        (
            1,
            claim(&key('b'), "Book", 10000),
            "legacy_binding_conflict",
        ),
        (
            1,
            claim(&key('a'), "Wrong", 10000),
            "legacy_binding_conflict",
        ),
        (
            2,
            claim(&key('a'), "Book", 10000),
            "legacy_binding_conflict",
        ),
        (1, claim(&key('a'), "Book", 9999), "book_length_mismatch"),
    ] {
        let r = f
            .server
            .put(&format!("/integrations/ebook/legacy-books/{id}"))
            .json(&b)
            .await;
        r.assert_status(StatusCode::CONFLICT);
        assert_eq!(r.json::<Value>()["error"]["code"], code);
        assert_eq!(snapshot(&f), before);
    }
    put(&f, P2, MON, &body(json!([[0, 150]]))).await;
    assert_eq!(amount(&f, MON).await, 140);
    put(&f, P2, MON, &body(json!([[0, 200]]))).await;
    assert_eq!(amount(&f, MON).await, 190);
    let mut phone = body(json!([[0, 80]]));
    phone["title"] = json!("Phone book 1");
    f.server
        .put(&path(P2, &key('b'), TUE))
        .json(&phone)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert_eq!(
        f.server.get("/progress-items/1").await.json::<Value>(),
        item
    );
    let items: Value = f.server.get("/progress-items").await.json();
    assert_eq!(items.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn adoption_failure_rolls_back_tags_binding_prefix_and_statistic() {
    let f = fixture();
    legacy_event(&f, "queued", 100, 80).await;
    let before = snapshot(&f);
    f.pool.get().unwrap().batch_execute("CREATE TRIGGER fail_claim BEFORE INSERT ON ebook_legacy_bindings BEGIN SELECT RAISE(ABORT,'injected claim failure'); END;").unwrap();
    let r = f
        .server
        .put("/integrations/ebook/legacy-books/1")
        .json(&claim(&key('a'), "Book", 10000))
        .await;
    r.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(r.json::<Value>()["error"]["code"], "storage_error");
    assert_eq!(snapshot(&f), before);
}

#[tokio::test]
async fn claims_validate_strictly_and_missing_legacy_items_get_independent_books() {
    let f = fixture();
    let before = snapshot(&f);
    for id in ["0", "-1", "4294967296", "x"] {
        let r = f
            .server
            .put(&format!("/integrations/ebook/legacy-books/{id}"))
            .json(&claim(&key('a'), "Book", 10000))
            .await;
        r.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(r.json::<Value>()["error"]["field"], "reader_data_id");
        assert_eq!(snapshot(&f), before);
    }
    let mut unknown = claim(&key('a'), "Book", 10000);
    unknown["x"] = json!(1);
    f.server
        .put("/integrations/ebook/legacy-books/1")
        .json(&unknown)
        .await
        .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    let duplicate = format!(
        r#"{{"book_key":"{}","title":"Book","length":10000,"length":10000}}"#,
        key('a')
    );
    f.server
        .put("/integrations/ebook/legacy-books/1")
        .text(&duplicate)
        .content_type("application/json")
        .await
        .assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(snapshot(&f), before);
    f.server
        .put("/integrations/ebook/legacy-books/99")
        .json(&claim(&key('a'), "Book", 10000))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    put(&f, P1, MON, &body(json!([[0, 100]]))).await;
    assert_eq!(amount(&f, MON).await, 100);
}

#[tokio::test]
#[ignore = "requires EBOOK_LIVE_BACKUP pointing to an isolated read-only backup source"]
async fn live_backup_adoption_preserves_43507_and_item_one() {
    let seed =
        std::env::var("EBOOK_LIVE_BACKUP").expect("create a read-only sqlite3 .backup first");
    assert_ne!(
        std::fs::canonicalize(&seed).unwrap(),
        PathBuf::from("/home/daniel/.local/share/src/lifestats/test_data/test_snapshot.db")
    );
    let db = std::env::temp_dir().join(format!(
        "lifestats-ebook-live-copy-{}.db",
        uuid::Uuid::new_v4()
    ));
    let status = std::process::Command::new("sqlite3")
        .arg(format!("file:{seed}?mode=ro"))
        .arg(format!(".backup '{}'", db.display()))
        .status()
        .unwrap();
    assert!(status.success());
    let f = fixture_at(db);
    let url = "/statistics/110/integer_amount/total?start_date=2026-01-01&end_date=2026-12-31";
    let before: Value = f.server.get(url).await.json();
    assert_eq!(before["total"], 43507);
    let item: Value = f.server.get("/progress-items/1").await.json();
    let title = item["tags"]["book_title"].as_str().unwrap();
    let length = item["target_value"].as_u64().unwrap();
    let mut report = body(json!([]));
    report["title"] = json!(title);
    report["length"] = json!(length);
    report["last_position"] = json!(0);
    let before_db = snapshot(&f);
    let r = f.server.put(&path(P2, &key('a'), MON)).json(&report).await;
    r.assert_status(StatusCode::CONFLICT);
    assert_eq!(
        r.json::<Value>()["error"]["code"],
        "legacy_adoption_pending"
    );
    assert_eq!(snapshot(&f), before_db);
    f.server
        .put("/integrations/ebook/legacy-books/1")
        .json(&claim(&key('a'), title, length))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert_eq!(f.server.get(url).await.json::<Value>(), before);
    let bound: Value = f.server.get("/progress-items/1").await.json();
    assert_eq!(bound["id"], 1);
    assert_eq!(bound["name"], item["name"]);
    assert_eq!(bound["tags"]["book_key"], key('a'));
    let prefix = query(&f, "SELECT json(legacy_spans) AS value FROM ebook_books");
    let furthest = prefix[0][1].as_u64().unwrap();
    assert!(furthest > 0);
    report["spans"] = json!([[0, furthest]]);
    f.server
        .put(&path(P2, &key('a'), MON))
        .json(&report)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert_eq!(f.server.get(url).await.json::<Value>(), before);
    let mut phone = body(json!([[0, 80]]));
    phone["title"] = json!("Different phone book 1");
    f.server
        .put(&path(P2, &key('b'), TUE))
        .json(&phone)
        .await
        .assert_status(StatusCode::NO_CONTENT);
    assert_eq!(
        f.server.get("/progress-items/1").await.json::<Value>(),
        bound
    );
    let all: Value = f.server.get("/progress-items").await.json();
    assert!(all
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["id"] != 1 && i["name"] == "Different phone book 1"));
    println!("VERIFIED live-copy: 2026 total 43507 before and after adoption; item 1 survives; pending gate clears; phone book independent; F={furthest}");
}

#[tokio::test]
async fn manual_overwrite_below_source_preserves_signed_residual_when_credit_moves() {
    let f = fixture();
    put(&f, P1, TUE, &body(json!([[0, 100]]))).await;
    f.server
        .post(&format!("/statistics/{}/log/{TUE}", statistic(&f)))
        .json(&json!({"value":"40","update_policy":"overwrite"}))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    put(&f, P1, TUE, &body(json!([[0, 150]]))).await;
    assert_eq!(amount(&f, TUE).await, 90);
    put(&f, P2, MON, &body(json!([[0, 100]]))).await;
    assert_eq!(amount(&f, MON).await, 100);
    assert_eq!(amount(&f, TUE).await, -10);
    let credits = query(
        &f,
        "SELECT json_group_object(day,characters) AS value FROM ebook_day_credits",
    );
    assert_eq!(credits, json!({MON:100,TUE:50}));
    assert_eq!(amount(&f, TUE).await, -60 + credits[TUE].as_i64().unwrap());
}

#[tokio::test]
async fn ambiguous_or_wrongly_typed_statistic_binding_fails_without_writes() {
    for wrong_type in [false, true] {
        let f = fixture();
        put(&f, P1, MON, &body(json!([[0, 100]]))).await;
        if wrong_type {
            f.pool.get().unwrap().batch_execute("UPDATE statistics SET statistic_type='Habit' WHERE json_extract(tags,'$.source')='ebook';").unwrap();
        } else {
            let metadata = f
                .repo
                .find_metadata_by_id(lifestats_core::shared::ids::StatisticId(statistic(&f)))
                .unwrap();
            f.repo.create_metadata(metadata).unwrap();
        }
        let before = snapshot(&f);
        let r = f
            .server
            .put(&path(P1, &key('a'), MON))
            .json(&body(json!([[0, 150]])))
            .await;
        r.assert_status(StatusCode::CONFLICT);
        assert_eq!(r.json::<Value>()["error"]["code"], "bundle_conflict");
        assert_eq!(snapshot(&f), before);
    }
}
