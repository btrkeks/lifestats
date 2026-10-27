//! Default goal history ("Applies from"), range overrides and per-day values,
//! against a real in-memory SQLite database.

use axum::http::StatusCode;
use axum_test::TestServer;
use chrono::{Local, NaiveDate};
use diesel::connection::SimpleConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use diesel_migrations::MigrationHarness;
use lifestats_application::use_cases::integration::IntegrationService;
use lifestats_application::SessionManager;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
use lifestats_core::statistic::types::logable_statistics::point_in_time::PointInTimeStatistic;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
use lifestats_persistence::{
    pool::MIGRATIONS, progress_repository::SqliteProgressRepository,
    repository::SqliteStatisticRepository,
};
use lifestats_plotting::PlottersRenderer;
use lifestats_web_api::{routes::create_router, state::AppState};
use serde_json::{json, Value};
use std::sync::Arc;

mod helpers;
use helpers::NoOpEventPublisher;

struct Fixture {
    server: TestServer,
    repo: Arc<SqliteStatisticRepository>,
}

fn fixture() -> Fixture {
    let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
    let pool = Arc::new(Pool::builder().max_size(1).build(manager).unwrap());
    let mut conn = pool.get().unwrap();
    conn.batch_execute("PRAGMA foreign_keys=ON;").unwrap();
    conn.run_pending_migrations(MIGRATIONS).unwrap();
    drop(conn);
    let repo = Arc::new(SqliteStatisticRepository::new(pool.clone()));
    let state = AppState::new(
        repo.clone(),
        Arc::new(SqliteProgressRepository::new(pool.clone())),
        Arc::new(NoOpEventPublisher),
        Arc::new(PlottersRenderer::new()),
        Arc::new(IntegrationService::new(repo.clone(), vec![])),
        Arc::new(SessionManager::new(repo.clone())),
    );
    Fixture {
        server: TestServer::new(create_router(state)).unwrap(),
        repo,
    }
}

fn date(month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, month, day).unwrap()
}

fn cards(f: &Fixture) -> StatisticId {
    let metadata = StatisticMetadata::new(StatisticId(0), "Yomikata".into(), "Cards".into());
    let stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(metadata, None, date(1, 1)));
    f.repo.create_metadata(stat).unwrap().get_metadata().id()
}

fn bedtime(f: &Fixture) -> StatisticId {
    let metadata = StatisticMetadata::new(StatisticId(0), "Health".into(), "Bedtime".into());
    let stat = AnyStatistic::PointInTime(PointInTimeStatistic::new(metadata, None, date(1, 1)));
    f.repo.create_metadata(stat).unwrap().get_metadata().id()
}

async fn set_default(f: &Fixture, id: StatisticId, goal_data: Value, valid_since: Option<&str>) {
    let mut body = json!({ "goal_data": goal_data });
    if let Some(valid_since) = valid_since {
        body["valid_since"] = json!(valid_since);
    }
    f.server
        .put(&format!("/statistics/{}/goals/default", id.0))
        .json(&body)
        .await
        .assert_status(StatusCode::NO_CONTENT);
}

async fn achievements(f: &Fixture, id: StatisticId, start: &str, end: &str) -> Vec<Value> {
    let body: Value = f
        .server
        .get(&format!(
            "/statistics/{}/achievements?start={}&end={}",
            id.0, start, end
        ))
        .await
        .json();
    body["days"].as_array().unwrap().clone()
}

#[tokio::test]
async fn changing_the_default_goal_keeps_earlier_days_on_the_old_goal() {
    let f = fixture();
    let id = cards(&f);
    for (day, amount) in [(1, 300), (2, 420), (3, 400), (4, 420)] {
        f.server
            .post(&format!("/statistics/{}/log/2026-10-0{}", id.0, day))
            .json(&json!({"value": amount.to_string()}))
            .await
            .assert_status(StatusCode::NO_CONTENT);
    }

    set_default(&f, id, json!({"target": "400"}), Some("2026-10-01")).await;
    set_default(&f, id, json!({"target": "450"}), Some("2026-10-03")).await;

    let days = achievements(&f, id, "2026-10-01", "2026-10-04").await;
    let targets: Vec<&Value> = days.iter().map(|d| &d["goal"]["target_amount"]).collect();
    assert_eq!(
        targets,
        [&json!(400), &json!(400), &json!(450), &json!(450)]
    );
    let values: Vec<&Value> = days.iter().map(|d| &d["value"]).collect();
    assert_eq!(values, [&json!(300), &json!(420), &json!(400), &json!(420)]);
    assert_eq!(days[1]["achieved"], true);
    assert_eq!(days[3]["achieved"], false);

    let current: Value = f
        .server
        .get(&format!("/statistics/{}/goals/default", id.0))
        .await
        .json();
    assert_eq!(
        current,
        json!({"goal_data": {"target_amount": 450, "is_at_least": true}, "valid_since": "2026-10-03"})
    );
}

#[tokio::test]
async fn an_earlier_applies_from_backfills_the_goal_and_keeps_overrides() {
    let f = fixture();
    let id = bedtime(&f);
    set_default(&f, id, json!({"deadline": "22:05"}), Some("2026-10-22")).await;
    f.server
        .put(&format!("/statistics/{}/goals/2026-10-17", id.0))
        .json(&json!({"kind": "suppressed"}))
        .await
        .assert_status(StatusCode::NO_CONTENT);

    set_default(&f, id, json!({"deadline": "22:05"}), Some("2026-10-12")).await;

    let days = achievements(&f, id, "2026-10-11", "2026-10-22").await;
    assert_eq!(
        days[0]["has_goal"], false,
        "the day before stays without goal"
    );
    assert_eq!(days[1]["goal"], json!({"target": "22:05:00"}));
    assert_eq!(days[6]["override"], "suppressed", "Oct 17 stays suppressed");
    assert_eq!(days[6]["has_goal"], false);
    let current: Value = f
        .server
        .get(&format!("/statistics/{}/goals/default", id.0))
        .await
        .json();
    assert_eq!(current["valid_since"], "2026-10-12");
}

#[tokio::test]
async fn deleting_the_default_goal_ends_it_today() {
    let f = fixture();
    let id = cards(&f);
    set_default(&f, id, json!({"target": "400"}), Some("2026-01-01")).await;

    f.server
        .delete(&format!("/statistics/{}/goals/default", id.0))
        .await
        .assert_status(StatusCode::NO_CONTENT);

    let current: Value = f
        .server
        .get(&format!("/statistics/{}/goals/default", id.0))
        .await
        .json();
    assert_eq!(current, Value::Null);
    let today = Local::now().date_naive();
    let yesterday = today.pred_opt().unwrap();
    let days = achievements(&f, id, &yesterday.to_string(), &today.to_string()).await;
    assert_eq!(days[0]["goal"]["target_amount"], 400);
    assert_eq!(days[1]["has_goal"], false);
}

#[tokio::test]
async fn range_sets_a_goal_or_suppresses_every_day() {
    let f = fixture();
    let id = bedtime(&f);
    f.server
        .post(&format!("/statistics/{}/log/2026-10-13", id.0))
        .json(&json!({"value": "22:37"}))
        .await
        .assert_status(StatusCode::NO_CONTENT);

    f.server
        .put(&format!("/statistics/{}/goals/range", id.0))
        .json(&json!({"start": "2026-10-12", "end": "2026-10-21", "kind": "goal", "goal_data": {"deadline": "22:05"}}))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    f.server
        .put(&format!("/statistics/{}/goals/range", id.0))
        .json(&json!({"start": "2026-10-20", "end": "2026-10-21", "kind": "suppressed"}))
        .await
        .assert_status(StatusCode::NO_CONTENT);

    let days = achievements(&f, id, "2026-10-11", "2026-10-22").await;
    let overrides: Vec<&str> = days
        .iter()
        .map(|d| d["override"].as_str().unwrap())
        .collect();
    assert_eq!(
        overrides,
        [
            "none",
            "goal",
            "goal",
            "goal",
            "goal",
            "goal",
            "goal",
            "goal",
            "goal",
            "suppressed",
            "suppressed",
            "none"
        ]
    );
    assert_eq!(days[2]["value"], "22:37:00");
    assert_eq!(days[2]["goal"], json!({"target": "22:05:00"}));
    assert_eq!(days[2]["achieved"], false);
}

#[tokio::test]
async fn range_rejects_bad_ranges_and_unknown_statistics() {
    let f = fixture();
    let id = bedtime(&f);
    let suppress =
        |start: &str, end: &str| json!({"start": start, "end": end, "kind": "suppressed"});

    f.server
        .put(&format!("/statistics/{}/goals/range", id.0))
        .json(&suppress("2026-10-21", "2026-10-12"))
        .await
        .assert_status(StatusCode::BAD_REQUEST);
    f.server
        .put(&format!("/statistics/{}/goals/range", id.0))
        .json(&suppress("2025-01-01", "2026-10-12"))
        .await
        .assert_status(StatusCode::BAD_REQUEST);
    f.server
        .put(&format!("/statistics/{}/goals/range", id.0))
        .json(&json!({"start": "2026-10-12", "end": "2026-10-13", "kind": "goal", "goal_data": {"deadline": "late"}}))
        .await
        .assert_status(StatusCode::BAD_REQUEST);
    f.server
        .put("/statistics/999/goals/range")
        .json(&suppress("2026-10-12", "2026-10-13"))
        .await
        .assert_status(StatusCode::NOT_FOUND);
}
