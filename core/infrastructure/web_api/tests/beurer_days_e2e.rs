use axum_test::TestServer;
use chrono::NaiveDate;
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::MigrationHarness;
use lifestats_application::use_cases::integration::IntegrationService;
use lifestats_application::SessionManager;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::types::logable_statistics::habit::HabitStatistic;
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
    let service =
        Arc::new(IntegrationService::new(repo.clone(), vec![]).with_beurer_day_store(repo.clone()));
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
fn date(day: &str) -> NaiveDate {
    day.parse().unwrap()
}
const DAY: &str = "2026-10-21";
fn path(day: &str) -> String {
    format!("/integrations/beurer/days/{day}")
}
async fn put(f: &Fixture, body: Value) {
    f.server
        .put(&path(DAY))
        .json(&body)
        .await
        .assert_status(axum::http::StatusCode::NO_CONTENT);
}
fn tagged(f: &Fixture, metric: &str) -> AnyStatistic {
    let statistics = f
        .repo
        .find_by_tags(&HashMap::from([
            ("source".into(), "beurer".into()),
            ("metric".into(), metric.into()),
        ]))
        .unwrap();
    assert_eq!(statistics.len(), 1);
    statistics.into_iter().next().unwrap()
}
fn value(f: &Fixture, metric: &str, day: &str) -> Option<Value> {
    f.repo
        .find_log_value(tagged(f, metric).get_metadata().id(), date(day))
        .unwrap()
}

#[tokio::test]
async fn provisions_four_tagged_statistics_and_overwrites_only_provided_levels() {
    let f = fixture();
    let body = json!({"weight_grams":67500,"body_fat_permille":156,"muscle_permille":426,"body_water_permille":609});
    put(&f, body.clone()).await;
    put(&f, body).await;
    assert_eq!(f.repo.find_all_metadata().unwrap().len(), 4);
    for (metric, name, amount) in [
        ("weight", "Weight (kg)", 67500),
        ("body_fat", "Body Fat (%)", 156),
        ("muscle", "Muscle (%)", 426),
        ("body_water", "Body Water (%)", 609),
    ] {
        let statistic = tagged(&f, metric);
        assert!(matches!(statistic, AnyStatistic::IntegerAmount(_)));
        assert_eq!(statistic.get_metadata().name(), format!("[Beurer] {name}"));
        assert_eq!(statistic.get_metadata().category(), "Beurer");
        assert_eq!(value(&f, metric, DAY), Some(json!(amount)));
    }
    // Neither names nor extra tags form part of the identity.
    let weight = tagged(&f, "weight");
    diesel::sql_query("UPDATE statistics SET name='Renamed weight' WHERE id=?")
        .bind::<diesel::sql_types::Integer, _>(weight.get_metadata().id().0 as i32)
        .execute(&mut f.pool.get().unwrap())
        .unwrap();
    put(
        &f,
        json!({"weight_grams":67000,"body_fat_permille":0,"muscle_permille":null}),
    )
    .await;
    assert_eq!(value(&f, "weight", DAY), Some(json!(67000)));
    assert_eq!(value(&f, "body_fat", DAY), Some(json!(0)));
    assert_eq!(value(&f, "muscle", DAY), Some(json!(426)));
    assert_eq!(value(&f, "body_water", DAY), Some(json!(609)));
    assert_eq!(
        tagged(&f, "weight").get_metadata().id(),
        weight.get_metadata().id()
    );
    f.server
        .put(&path("2026-10-22"))
        .json(&json!({"weight_grams":500000}))
        .await
        .assert_status(axum::http::StatusCode::NO_CONTENT);
    assert_eq!(value(&f, "weight", DAY), Some(json!(67000)));
    assert_eq!(value(&f, "muscle", "2026-10-22"), None);
}

#[tokio::test]
async fn weight_only_first_call_still_provisions_all_four() {
    let f = fixture();
    put(&f, json!({"weight_grams":1})).await;
    assert_eq!(f.repo.find_all_metadata().unwrap().len(), 4);
    assert_eq!(value(&f, "body_fat", DAY), None);
    put(&f,json!({"weight_grams":500000,"body_fat_permille":1000,"muscle_permille":0,"body_water_permille":1000})).await;
}

#[tokio::test]
async fn invalid_requests_share_yomikata_status_and_error_envelope_without_writes() {
    let f = fixture();
    for (body, field) in [
        ("{}", "weight_grams"),
        (r#"{"weight_grams":null}"#, "weight_grams"),
        (r#"{"weight_grams":0}"#, "weight_grams"),
        (r#"{"weight_grams":500001}"#, "weight_grams"),
        (r#"{"weight_grams":67500.0}"#, "weight_grams"),
        (r#"{"weight_grams":"67500"}"#, "weight_grams"),
        (r#"{"weight_grams":true}"#, "weight_grams"),
        (r#"{"weight_grams":1e400}"#, "weight_grams"),
        (
            r#"{"weight_grams":67500,"body_fat_permille":-1}"#,
            "body_fat_permille",
        ),
        (
            r#"{"weight_grams":67500,"muscle_permille":1001}"#,
            "muscle_permille",
        ),
        (
            r#"{"weight_grams":67500,"body_water_permille":1.0}"#,
            "body_water_permille",
        ),
        (
            r#"{"weight_grams":67500,"body_fat_permille":-0}"#,
            "body_fat_permille",
        ),
        (
            r#"{"weight_grams":67500,"body_water_permille":999999999999999999999999}"#,
            "body_water_permille",
        ),
        (
            r#"{"weight_grams":{"$serde_json::private::Number":"67500"}}"#,
            "weight_grams",
        ),
    ] {
        let response = f
            .server
            .put(&path(DAY))
            .text(body)
            .content_type("application/json")
            .await;
        response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        let error = response.json::<Value>();
        assert_eq!(error["error"]["code"], "invalid_field", "{body}");
        assert_eq!(error["error"]["field"], field, "{body}");
        assert!(error["error"]["message"].is_string());
    }
    for body in [
        r#"{"weight_grams":1,"extra":1}"#,
        r#"{"weight_grams":1,"weight_grams":2}"#,
        r#"{"weight_grams":1,"body_fat_permille":null,"body_fat_permille":1}"#,
        r#"{"weight_grams":1,"muscle_permille":0,"muscle_permille":0}"#,
        r#"{"weight_grams":1,"body_water_permille":0,"body_water_\u0070ermille":1}"#,
        "[]",
        "null",
    ] {
        let response = f
            .server
            .put(&path(DAY))
            .text(body)
            .content_type("application/json")
            .await;
        response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(response.json::<Value>()["error"]["code"], "invalid_field");
    }
    for day in [
        "2026-02-29",
        "2026-1-01",
        "26-10-21",
        "2026-10-21x",
        "+2026-10-21",
    ] {
        let response = f
            .server
            .put(&path(day))
            .json(&json!({"weight_grams":1}))
            .await;
        response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(response.json::<Value>()["error"]["field"], "day");
    }
    for (body, content_type, status, code) in [
        ("{", "application/json", 400, "malformed_json"),
        ("{}", "text/plain", 415, "unsupported_media_type"),
    ] {
        let response = f
            .server
            .put(&path(DAY))
            .text(body)
            .content_type(content_type)
            .await;
        assert_eq!(response.status_code().as_u16(), status);
        assert_eq!(response.json::<Value>()["error"]["code"], code);
    }
    assert!(f.repo.find_all_metadata().unwrap().is_empty());
}

#[tokio::test]
async fn duplicate_or_wrongly_typed_tags_conflict_and_roll_back_all_metrics() {
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
    for duplicate in [false, true] {
        let f = fixture();
        let metadata = StatisticMetadata::new(StatisticId(0), "Other".into(), "Conflict".into())
            .with_tag("source".into(), "beurer".into())
            .with_tag("metric".into(), "body_water".into());
        let statistic = if duplicate {
            AnyStatistic::IntegerAmount(lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic::new(metadata,None,date(DAY)))
        } else {
            AnyStatistic::Habit(HabitStatistic::new(metadata, None, date(DAY)))
        };
        f.repo.create_metadata(statistic.clone()).unwrap();
        if duplicate {
            f.repo.create_metadata(statistic).unwrap();
        }
        let count = f.repo.find_all_metadata().unwrap().len();
        let response = f
            .server
            .put(&path(DAY))
            .json(&json!({"weight_grams":67500}))
            .await;
        response.assert_status(axum::http::StatusCode::CONFLICT);
        assert_eq!(response.json::<Value>()["error"]["code"], "bundle_conflict");
        assert_eq!(f.repo.find_all_metadata().unwrap().len(), count);
    }
    let f = fixture();
    put(&f, json!({"weight_grams":67500,"body_water_permille":609})).await;
    f.repo.create_metadata(tagged(&f, "body_water")).unwrap();
    f.server
        .put(&path(DAY))
        .json(&json!({"weight_grams":66000}))
        .await
        .assert_status(axum::http::StatusCode::CONFLICT);
    assert_eq!(value(&f, "weight", DAY), Some(json!(67500)));
}

#[tokio::test]
async fn storage_errors_are_sanitized_and_roll_back_provisioning_and_partial_updates() {
    for existing in [false, true] {
        let f = fixture();
        if existing {
            put(&f, json!({"weight_grams":67500,"body_fat_permille":156})).await;
        }
        f.pool.get().unwrap().batch_execute("CREATE TRIGGER fail_beurer BEFORE INSERT ON logging_data WHEN NEW.value='155' BEGIN SELECT RAISE(ABORT,'private storage details'); END;").unwrap();
        let response = f
            .server
            .put(&path(DAY))
            .json(&json!({"weight_grams":67000,"body_fat_permille":155}))
            .await;
        response.assert_status(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            response.json::<Value>(),
            json!({"error":{"code":"storage_error","message":"Unable to store daily totals"}})
        );
        if existing {
            assert_eq!(value(&f, "weight", DAY), Some(json!(67500)));
            assert_eq!(value(&f, "body_fat", DAY), Some(json!(156)));
        } else {
            assert!(f.repo.find_all_metadata().unwrap().is_empty());
        }
    }
}

#[tokio::test]
async fn plot_data_and_png_use_human_units_without_changing_storage_units() {
    let f = fixture();
    put(&f,json!({"weight_grams":67500,"body_fat_permille":156,"muscle_permille":426,"body_water_permille":609})).await;
    for (metric, stored, human, unit, format) in [
        ("weight", 67500, 67.5, "kilograms", "kilograms_one_decimal"),
        (
            "body_fat",
            156,
            15.6,
            "percent_one_decimal",
            "percent_one_decimal",
        ),
        (
            "muscle",
            426,
            42.6,
            "percent_one_decimal",
            "percent_one_decimal",
        ),
        (
            "body_water",
            609,
            60.9,
            "percent_one_decimal",
            "percent_one_decimal",
        ),
    ] {
        let id = tagged(&f, metric).get_metadata().id().0;
        let response = f
            .server
            .get(&format!(
                "/statistics/{id}/plot/data?plot_type=line&date_from={DAY}&date_to={DAY}"
            ))
            .await;
        response.assert_status_success();
        let data = response.json::<Value>();
        assert_eq!(
            data["points"][0]["value"]["value"],
            json!({"value":human,"unit":unit})
        );
        assert_eq!(data["y_axis"]["label_format"], format);
        assert!(data["y_axis"]["max"].as_f64().unwrap() >= human);
        assert!(data["y_axis"]["max"].as_f64().unwrap() <= 100.0);
        assert_eq!(value(&f, metric, DAY), Some(json!(stored)));
        let image=f.server.get(&format!("/statistics/{id}/plot/image?plot_type=line&date_from={DAY}&date_to={DAY}&width=640&height=480")).await;
        image.assert_status_success();
        assert!(image.as_bytes().starts_with(b"\x89PNG\r\n\x1a\n"));
    }
}

#[tokio::test]
async fn generated_openapi_describes_beurer_validation_and_statuses() {
    let f = fixture();
    let doc = f.server.get("/openapi.json").await.json::<Value>();
    let put = &doc["paths"]["/integrations/beurer/days/{day}"]["put"];
    for status in ["204", "400", "409", "415", "422", "500"] {
        assert!(put["responses"][status].is_object());
    }
    let schema = &doc["components"]["schemas"]["BeurerDayRequest"];
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["required"], json!(["weight_grams"]));
    assert_eq!(schema["properties"]["weight_grams"]["minimum"], 1);
    assert_eq!(schema["properties"]["weight_grams"]["maximum"], 500000);
    for field in [
        "body_fat_permille",
        "muscle_permille",
        "body_water_permille",
    ] {
        assert_eq!(schema["properties"][field]["minimum"], 0);
        assert_eq!(schema["properties"][field]["maximum"], 1000);
    }
}
