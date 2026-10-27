use axum::http::StatusCode;
use axum_test::TestServer;
use chrono::NaiveDate;
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
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
use lifestats_persistence::{
    pool::MIGRATIONS, progress_repository::SqliteProgressRepository,
    repository::SqliteStatisticRepository,
};
use lifestats_plotting::PlottersRenderer;
use lifestats_web_api::{routes::create_router, state::AppState};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};
mod helpers;
use helpers::NoOpEventPublisher;

struct Fixture {
    server: TestServer,
    repo: Arc<SqliteStatisticRepository>,
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
}

fn fixture() -> Fixture {
    let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
    let pool = Arc::new(Pool::builder().max_size(1).build(manager).unwrap());
    let mut conn = pool.get().unwrap();
    conn.batch_execute("PRAGMA foreign_keys=ON;").unwrap();
    conn.run_pending_migrations(MIGRATIONS).unwrap();
    drop(conn);
    let repo = Arc::new(SqliteStatisticRepository::new(pool.clone()));
    let service = Arc::new(
        IntegrationService::new(repo.clone(), vec![]).with_opennutritracker_day_store(repo.clone()),
    );
    let state = AppState::new(
        repo.clone(),
        Arc::new(SqliteProgressRepository::new(pool.clone())),
        Arc::new(NoOpEventPublisher),
        Arc::new(PlottersRenderer::new()),
        service,
        Arc::new(SessionManager::new(repo.clone())),
    );
    Fixture {
        server: TestServer::builder()
            .http_transport()
            .build(create_router(state))
            .unwrap(),
        repo,
        pool,
    }
}

const DAY: &str = "2026-10-27";

fn date(day: &str) -> NaiveDate {
    day.parse().unwrap()
}

fn path(day: &str) -> String {
    format!("/integrations/opennutritracker/days/{day}")
}

async fn put(f: &Fixture, day: &str, body: Value) {
    f.server
        .put(&path(day))
        .json(&body)
        .await
        .assert_status(StatusCode::NO_CONTENT);
}

fn tagged(f: &Fixture, metric: &str) -> Option<AnyStatistic> {
    let mut statistics = f
        .repo
        .find_by_tags(&HashMap::from([
            ("source".into(), "opennutritracker".into()),
            ("metric".into(), metric.into()),
        ]))
        .unwrap();
    assert!(statistics.len() <= 1);
    statistics.pop()
}

fn id(f: &Fixture, metric: &str) -> u32 {
    tagged(f, metric).unwrap().get_metadata().id().0
}

fn value(f: &Fixture, metric: &str, day: &str) -> Option<Value> {
    f.repo
        .find_log_value(StatisticId(id(f, metric)), date(day))
        .unwrap()
}

fn coffee_day() -> Value {
    json!({
        "caffeine": [
            {"time": "13:40:00", "amount_mg": 60},
            {"time": "08:12:00", "amount_mg": 120},
            {"time": null, "amount_mg": 40}
        ],
        "nicotine": []
    })
}

#[tokio::test]
async fn provisions_both_metrics_and_replaces_whole_days() {
    let f = fixture();
    put(&f, DAY, coffee_day()).await;
    put(&f, DAY, coffee_day()).await;

    assert_eq!(f.repo.find_all_metadata().unwrap().len(), 2);
    for (metric, name) in [
        ("caffeine", "[OpenNutriTracker] Caffeine (mg)"),
        ("nicotine", "[OpenNutriTracker] Nicotine (mg)"),
    ] {
        let statistic = tagged(&f, metric).unwrap();
        assert!(matches!(statistic, AnyStatistic::TimedAmounts(_)));
        assert_eq!(statistic.get_metadata().name(), name);
        assert_eq!(statistic.get_metadata().category(), "OpenNutriTracker");
        assert_eq!(statistic.get_default_goal().map(|_| ()), None);
    }
    // Stored canonically: unknown time first, then by time.
    assert_eq!(
        value(&f, "caffeine", DAY),
        Some(json!([
            {"time": null, "amount": 40},
            {"time": "08:12:00", "amount": 120},
            {"time": "13:40:00", "amount": 60}
        ]))
    );
    assert_eq!(value(&f, "nicotine", DAY), Some(json!([])));

    let logged = f
        .server
        .get(&format!("/statistics/{}/logged/{DAY}", id(&f, "caffeine")))
        .await
        .json::<Value>();
    assert_eq!(logged["has_value"], json!(true));
    assert_eq!(
        logged["value"][1],
        json!({"time": "08:12:00", "amount": 120})
    );

    // An edit removes the old entry; an empty list is a durable zero; other days stay.
    put(
        &f,
        "2026-10-26",
        json!({"caffeine": [{"time": "09:00:00", "amount_mg": 95}], "nicotine": []}),
    )
    .await;
    put(&f, DAY, json!({"caffeine": [], "nicotine": [{"time": "10:00:00", "amount_mg": 2}, {"time": "10:00:00", "amount_mg": 2}]})).await;
    assert_eq!(value(&f, "caffeine", DAY), Some(json!([])));
    assert_eq!(
        value(&f, "nicotine", DAY),
        Some(json!([{"time": "10:00:00", "amount": 2}, {"time": "10:00:00", "amount": 2}]))
    );
    assert_eq!(
        value(&f, "caffeine", "2026-10-26"),
        Some(json!([{"time": "09:00:00", "amount": 95}]))
    );
    assert_eq!(value(&f, "nicotine", "2026-10-25"), None);
}

#[tokio::test]
async fn invalid_requests_are_rejected_before_any_write() {
    let f = fixture();
    let entry = |time: Value, amount: Value| json!({"caffeine": [{"time": time, "amount_mg": amount}], "nicotine": []});
    let cases: Vec<(Value, StatusCode, &str, Option<&str>)> = vec![
        (
            json!({"caffeine": []}),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_field",
            Some("nicotine"),
        ),
        (
            json!({"caffeine": null, "nicotine": []}),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_field",
            Some("caffeine"),
        ),
        (
            json!({"caffeine": {}, "nicotine": []}),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_field",
            Some("caffeine"),
        ),
        (
            json!({"caffeine": [], "nicotine": [], "alcohol": []}),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_field",
            None,
        ),
        (
            json!([]),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_field",
            None,
        ),
        (
            json!({"caffeine": [5], "nicotine": []}),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_field",
            Some("caffeine[0]"),
        ),
        (
            json!({"caffeine": [{"amount_mg": 5}], "nicotine": []}),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_field",
            Some("caffeine[0].time"),
        ),
        (
            entry(json!("08:12"), json!(5)),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_field",
            Some("caffeine[0].time"),
        ),
        (
            entry(json!("24:00:00"), json!(5)),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_field",
            Some("caffeine[0].time"),
        ),
        (
            entry(json!(800), json!(5)),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_field",
            Some("caffeine[0].time"),
        ),
        (
            entry(json!(null), json!(0)),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_field",
            Some("caffeine[0].amount_mg"),
        ),
        (
            entry(json!(null), json!(-3)),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_field",
            Some("caffeine[0].amount_mg"),
        ),
        (
            entry(json!(null), json!(120.0)),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_field",
            Some("caffeine[0].amount_mg"),
        ),
        (
            entry(json!(null), json!("120")),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_field",
            Some("caffeine[0].amount_mg"),
        ),
        (
            json!({"caffeine": [{"time": null, "amount_mg": 5, "unit": "mg"}], "nicotine": []}),
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_field",
            Some("caffeine[0]"),
        ),
        (
            json!({"caffeine": [{"time": null, "amount_mg": 9007199254740991_i64}, {"time": null, "amount_mg": 1}], "nicotine": []}),
            StatusCode::UNPROCESSABLE_ENTITY,
            "projection_overflow",
            Some("caffeine"),
        ),
    ];
    for (body, status, code, field) in cases {
        let response = f.server.put(&path(DAY)).json(&body).await;
        response.assert_status(status);
        let error = &response.json::<Value>()["error"];
        assert_eq!(error["code"], json!(code), "{body}");
        assert_eq!(error.get("field").and_then(Value::as_str), field, "{body}");
    }

    let too_many: Vec<Value> = (0..1001)
        .map(|_| json!({"time": null, "amount_mg": 1}))
        .collect();
    let response = f
        .server
        .put(&path(DAY))
        .json(&json!({"caffeine": too_many, "nicotine": []}))
        .await;
    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        response.json::<Value>()["error"]["field"],
        json!("caffeine")
    );

    for raw in [
        r#"{"caffeine":[],"nicotine":[],"nicotine":[]}"#,
        r#"{"caffeine":[{"time":null,"amount_mg":1,"amount_mg":2}],"nicotine":[]}"#,
        r#"{"caffeine":[{"time":null,"amount_mg":1e2}],"nicotine":[]}"#,
    ] {
        let response = f
            .server
            .put(&path(DAY))
            .content_type("application/json")
            .bytes(raw.as_bytes().to_vec().into())
            .await;
        response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let response = f
        .server
        .put(&path(DAY))
        .content_type("application/json")
        .bytes(b"{nope".to_vec().into())
        .await;
    response.assert_status(StatusCode::BAD_REQUEST);
    assert_eq!(
        response.json::<Value>()["error"]["code"],
        json!("malformed_json")
    );
    let response = f.server.put(&path(DAY)).text("{}").await;
    response.assert_status(StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let response = f.server.put(&path("2026-02-30")).json(&coffee_day()).await;
    response.assert_status(StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(response.json::<Value>()["error"]["field"], json!("day"));

    assert!(f.repo.find_all_metadata().unwrap().is_empty());
}

#[tokio::test]
async fn conflicting_bindings_and_storage_failures_roll_back() {
    let f = fixture();
    let impostor = StatisticMetadata::new(StatisticId(0), "x".into(), "Nicotine".into())
        .with_tag("source".into(), "opennutritracker".into())
        .with_tag("metric".into(), "nicotine".into());
    f.repo
        .create_metadata(AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            impostor,
            None,
            date(DAY),
        )))
        .unwrap();
    let response = f.server.put(&path(DAY)).json(&coffee_day()).await;
    response.assert_status(StatusCode::CONFLICT);
    assert_eq!(
        response.json::<Value>(),
        json!({"error": {"code": "bundle_conflict", "message": "OpenNutriTracker statistics require correction"}})
    );
    assert!(tagged(&f, "caffeine").is_none());

    let f = fixture();
    put(&f, DAY, coffee_day()).await;
    f.pool
        .get()
        .unwrap()
        .batch_execute(
            "CREATE TRIGGER fail_nicotine BEFORE INSERT ON logging_data WHEN NEW.value LIKE '%\"amount\":4}%' \
             BEGIN SELECT RAISE(ABORT,'private storage details'); END;",
        )
        .unwrap();
    let response = f
        .server
        .put(&path(DAY))
        .json(&json!({"caffeine": [], "nicotine": [{"time": null, "amount_mg": 4}]}))
        .await;
    response.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        response.json::<Value>(),
        json!({"error": {"code": "storage_error", "message": "Unable to store OpenNutriTracker day"}})
    );
    // The caffeine write before the failing nicotine write rolled back too.
    assert_eq!(
        value(&f, "caffeine", DAY)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        3
    );
}

#[tokio::test]
async fn reads_project_the_daily_total_and_goals_cap_it() {
    let f = fixture();
    put(&f, "2026-10-25", json!({"caffeine": [{"time": "08:00:00", "amount_mg": 300}, {"time": "15:00:00", "amount_mg": 200}], "nicotine": []})).await;
    put(&f, "2026-10-26", json!({"caffeine": [], "nicotine": []})).await;
    put(&f, DAY, coffee_day()).await;
    let caffeine = id(&f, "caffeine");

    f.server
        .put(&format!("/statistics/{caffeine}/goals/default"))
        .json(&json!({"goal_data": {"target": "400", "is_at_least": "false"}, "valid_since": "2026-10-01"}))
        .await
        .assert_status(StatusCode::NO_CONTENT);
    f.server
        .put(&format!("/statistics/{caffeine}/goals/default"))
        .json(&json!({"goal_data": {"target": "-1", "is_at_least": "false"}, "valid_since": "2026-10-01"}))
        .await
        .assert_status(StatusCode::BAD_REQUEST);

    let achievements = f
        .server
        .get(&format!(
            "/statistics/{caffeine}/achievements?start=2026-10-24&end=2026-10-27"
        ))
        .await
        .json::<Value>();
    let days = achievements["days"].as_array().unwrap();
    // Never sent: no value, goal missed. [] confirmed zero: met. Over the cap: missed.
    assert_eq!(days[0]["value"], json!(null));
    assert_eq!(days[0]["achieved"], json!(false));
    assert_eq!(
        days[0]["goal"],
        json!({"target_amount": 400, "is_at_least": false})
    );
    assert_eq!(
        (days[1]["value"].clone(), days[1]["achieved"].clone()),
        (json!(500), json!(false))
    );
    assert_eq!(
        (days[2]["value"].clone(), days[2]["achieved"].clone()),
        (json!(0), json!(true))
    );
    assert_eq!(
        (days[3]["value"].clone(), days[3]["achieved"].clone()),
        (json!(220), json!(true))
    );

    let total = f
        .server
        .get(&format!(
            "/statistics/{caffeine}/integer_amount/total?start_date=2026-10-24&end_date=2026-10-27"
        ))
        .await
        .json::<Value>();
    assert_eq!(total, json!({"total": 720}));

    let dashboard = f
        .server
        .post("/statistics/dashboard")
        .json(&json!({"statistic_ids": [caffeine], "today": DAY, "performance_window_days": 7}))
        .await
        .json::<Value>();
    assert_eq!(
        dashboard["statistics"][0]["statistic_type"],
        json!("TimedAmounts")
    );
    assert_eq!(dashboard["statistics"][0]["today_value"], json!(220));

    f.server
        .get(&format!(
            "/statistics/{caffeine}/score?start=2026-10-24&end=2026-10-27"
        ))
        .await
        .assert_status_success();

    let plot = f
        .server
        .get(&format!(
            "/statistics/{caffeine}/plot/data?plot_type=bar&date_from=2026-10-25&date_to={DAY}"
        ))
        .await
        .json::<Value>();
    let values: Vec<Value> = plot["points"]
        .as_array()
        .unwrap()
        .iter()
        .map(|point| point["value"]["value"]["value"].clone())
        .collect();
    assert_eq!(values, vec![json!(500), json!(0), json!(220)]);
    let image = f
        .server
        .get(&format!("/statistics/{caffeine}/plot/image?plot_type=line&date_from=2026-10-25&date_to={DAY}&width=640&height=480"))
        .await;
    image.assert_status_success();
    assert!(image.as_bytes().starts_with(b"\x89PNG\r\n\x1a\n"));
    for plot_type in ["time_frames", "circular_progress"] {
        f.server
            .get(&format!("/statistics/{caffeine}/plot/data?plot_type={plot_type}&date_from=2026-09-01&date_to=2026-09-02"))
            .await
            .assert_status(StatusCode::BAD_REQUEST);
    }

    let summary = f.server.get(&format!("/daily_summary/{DAY}")).await;
    summary.assert_status_success();
    assert_eq!(summary.json::<Value>()["timeline_events"], json!([]));
}

#[tokio::test]
async fn generic_writes_reject_the_source_owned_type() {
    let f = fixture();
    put(&f, DAY, coffee_day()).await;
    let caffeine = id(&f, "caffeine");

    let listed = f.server.get("/statistics").await.json::<Value>();
    let types: Vec<Value> = listed
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["statistic_type"].clone())
        .collect();
    assert_eq!(types, vec![json!("TimedAmounts"), json!("TimedAmounts")]);
    assert_eq!(
        f.server.get("/statistics/loggable").await.json::<Value>(),
        json!([])
    );

    let response = f
        .server
        .post(&format!("/statistics/{caffeine}/log/{DAY}"))
        .json(&json!({"value": "[]", "update_policy": "overwrite"}))
        .await;
    response.assert_status(StatusCode::BAD_REQUEST);
    let response = f
        .server
        .post("/statistics")
        .json(&json!({"name": "Caffeine", "category": "Me", "statistic_type": "TimedAmounts"}))
        .await;
    response.assert_status(StatusCode::BAD_REQUEST);
    assert_eq!(f.repo.find_all_metadata().unwrap().len(), 2);
}

#[tokio::test]
async fn openapi_describes_required_nullable_times() {
    let f = fixture();
    let doc = f.server.get("/openapi.json").await.json::<Value>();
    assert!(doc["paths"]["/integrations/opennutritracker/days/{day}"]["put"].is_object());
    let schemas = &doc["components"]["schemas"];
    let entry = &schemas["TimedMilligramsRequest"];
    assert_eq!(entry["required"], json!(["time", "amount_mg"]));
    let day = &schemas["OpenNutriTrackerDayRequest"];
    assert_eq!(day["required"], json!(["caffeine", "nicotine"]));
    assert_eq!(day["properties"]["caffeine"]["maxItems"], json!(1000));
}
