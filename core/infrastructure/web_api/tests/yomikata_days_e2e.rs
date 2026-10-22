use axum_test::TestServer;
use chrono::NaiveDate;
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::MigrationHarness;
use lifestats_application::use_cases::integration::{
    BundleFactory, IntegrationService, YomikataBundleFactory,
};
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
    let service = Arc::new(
        IntegrationService::new(
            repo.clone(),
            vec![Arc::new(YomikataBundleFactory)],
            None,
            None,
        )
        .with_yomikata_day_store(repo.clone()),
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
fn date(day: &str) -> NaiveDate {
    day.parse().unwrap()
}
const DAY: &str = "2026-10-21";
fn path(day: &str) -> String {
    format!("/integrations/yomikata/days/{day}")
}
async fn put(f: &Fixture, cards: i64, seconds: i64) {
    f.server
        .put(&path(DAY))
        .json(&json!({"cards_reviewed":cards,"study_time_seconds":seconds}))
        .await
        .assert_status(axum::http::StatusCode::NO_CONTENT);
}
fn ids(f: &Fixture) -> (StatisticId, StatisticId) {
    let find = |metric: &str| {
        let values = f
            .repo
            .find_by_tags(&HashMap::from([
                ("source".into(), "yomikata".into()),
                ("metric".into(), metric.into()),
            ]))
            .unwrap();
        assert_eq!(values.len(), 1);
        values[0].get_metadata().id()
    };
    (find("cards_reviewed"), find("study_minutes"))
}
fn value(f: &Fixture, id: StatisticId, day: &str) -> Value {
    f.repo.find_log_value(id, date(day)).unwrap().unwrap()
}
fn legacy(f: &Fixture, count: usize) -> Vec<StatisticId> {
    YomikataBundleFactory
        .provision(&HashMap::new())
        .unwrap()
        .into_iter()
        .take(count)
        .map(|p| {
            f.repo
                .create_metadata(p.statistic)
                .unwrap()
                .get_metadata()
                .id()
        })
        .collect()
}
async fn manual(f: &Fixture, id: StatisticId, amount: i64, policy: &str) {
    f.server
        .post(&format!("/statistics/{}/log/{DAY}", id.0))
        .json(&json!({"value": amount.to_string(), "update_policy":policy}))
        .await
        .assert_status_success();
}

#[tokio::test]
async fn replaces_duplicates_lowered_zero_and_exact_second_boundaries() {
    let f = fixture();
    put(&f, 42, 932).await;
    let (cards, minutes) = ids(&f);
    assert_eq!(value(&f, cards, DAY), 42);
    assert_eq!(value(&f, minutes, DAY), 15);
    for seconds in [932, 59, 60, 61, 119, 0, 0] {
        put(&f, 8, seconds).await;
        assert_eq!(value(&f, cards, DAY), 8);
        assert_eq!(value(&f, minutes, DAY), seconds / 60);
    }
    put(&f, 0, 0).await;
    put(&f, 0, 0).await;
    assert_eq!(value(&f, cards, DAY), 0);
    f.repo
        .save_log_value(cards, date("2026-10-20"), json!(7))
        .unwrap();
    put(&f, 1, 61).await;
    assert_eq!(value(&f, cards, "2026-10-20"), 7);
}

#[tokio::test]
async fn preserves_manual_residual_and_aggregate_overwrite_semantics() {
    let f = fixture();
    put(&f, 10, 600).await;
    let (cards, minutes) = ids(&f);
    manual(&f, cards, 5, "sum").await;
    manual(&f, minutes, 2, "sum").await;
    put(&f, 8, 480).await;
    assert_eq!(value(&f, cards, DAY), 13);
    assert_eq!(value(&f, minutes, DAY), 10);
    manual(&f, cards, 4, "overwrite").await;
    manual(&f, cards, 999, "ignore").await;
    put(&f, 0, 0).await;
    assert_eq!(value(&f, cards, DAY), -4);
    assert_eq!(value(&f, minutes, DAY), 2);
    put(&f, 0, 0).await;
    assert_eq!(value(&f, cards, DAY), -4);
}

#[tokio::test]
async fn adopts_all_legacy_days_once_preserving_ids_and_goals() {
    let f = fixture();
    let original = legacy(&f, 2);
    let (cards, minutes) = (original[0], original[1]);
    let earlier = "2026-01-01";
    for day in [DAY, earlier] {
        f.repo.save_log_value(cards, date(day), json!(10)).unwrap();
        f.repo.save_log_value(minutes, date(day), json!(3)).unwrap();
    }
    f.repo
        .save_goal(cards, date(DAY), Some(json!({"target":25})))
        .unwrap();
    let default = f.repo.find_default_goal(cards).unwrap();
    let goal = f.repo.find_goal(cards, date(DAY)).unwrap();
    put(&f, 12, 240).await;
    assert_eq!(ids(&f), (cards, minutes));
    assert_eq!(value(&f, cards, DAY), 12);
    assert_eq!(value(&f, minutes, DAY), 4);
    assert_eq!(value(&f, cards, earlier), 10);
    assert_eq!(value(&f, minutes, earlier), 3);
    assert_eq!(f.repo.find_default_goal(cards).unwrap(), default);
    assert_eq!(
        format!("{:?}", f.repo.find_goal(cards, date(DAY)).unwrap()),
        format!("{goal:?}")
    );
    f.server
        .put(&path(earlier))
        .json(&json!({"cards_reviewed":0,"study_time_seconds":0}))
        .await
        .assert_status_success();
    manual(&f, cards, 5, "sum").await;
    put(&f, 0, 0).await;
    put(&f, 0, 0).await;
    assert_eq!(value(&f, cards, DAY), 5);
    assert_eq!(value(&f, cards, earlier), 0);
    assert_eq!(value(&f, minutes, earlier), 0);
}

#[tokio::test]
async fn partial_bundle_is_adopted_and_deleted_statistics_rebuild_all_source_days() {
    let f = fixture();
    let cards = legacy(&f, 1)[0];
    f.repo.save_log_value(cards, date(DAY), json!(20)).unwrap();
    put(&f, 21, 61).await;
    let (adopted, minutes) = ids(&f);
    assert_eq!(adopted, cards);
    assert_eq!(value(&f, cards, DAY), 21);
    f.server
        .put(&path("2026-10-20"))
        .json(&json!({"cards_reviewed":7,"study_time_seconds":119}))
        .await
        .assert_status_success();
    f.repo.delete_metadata(minutes).unwrap();
    put(&f, 22, 121).await;
    let (_, new_minutes) = ids(&f);
    assert_eq!(value(&f, new_minutes, DAY), 2);
    assert_eq!(value(&f, new_minutes, "2026-10-20"), 1);
    f.repo.delete_metadata(cards).unwrap();
    put(&f, 22, 121).await;
    let (new_cards, _) = ids(&f);
    assert_eq!(value(&f, new_cards, DAY), 22);
    assert_eq!(value(&f, new_cards, "2026-10-20"), 7);
}

#[tokio::test]
async fn ambiguous_or_wrongly_typed_tags_conflict_without_provisioning() {
    for wrong_type in [false, true] {
        let f = fixture();
        let cards = legacy(&f, 1)[0];
        let stat = f.repo.find_metadata_by_id(cards).unwrap();
        let another = if wrong_type {
            AnyStatistic::Habit(HabitStatistic::new(
                stat.get_metadata().clone(),
                None,
                date(DAY),
            ))
        } else {
            stat
        };
        f.repo.create_metadata(another).unwrap();
        let response = f
            .server
            .put(&path(DAY))
            .json(&json!({"cards_reviewed":1,"study_time_seconds":1}))
            .await;
        response.assert_status(axum::http::StatusCode::CONFLICT);
        assert_eq!(response.json::<Value>()["error"]["code"], "bundle_conflict");
        assert_eq!(f.repo.find_all_metadata().unwrap().len(), 2);
    }
    let f = fixture();
    let cards = legacy(&f, 1)[0];
    let stat = f.repo.find_metadata_by_id(cards).unwrap();
    f.repo.delete_metadata(cards).unwrap();
    f.repo
        .create_metadata(AnyStatistic::Habit(HabitStatistic::new(
            stat.get_metadata().clone(),
            None,
            date(DAY),
        )))
        .unwrap();
    f.server
        .put(&path(DAY))
        .json(&json!({"cards_reviewed":1,"study_time_seconds":1}))
        .await
        .assert_status(axum::http::StatusCode::CONFLICT);
    assert_eq!(f.repo.find_all_metadata().unwrap().len(), 1);
}

#[tokio::test]
async fn projection_failure_rolls_back_day_both_values_and_fresh_provisioning() {
    let f = fixture();
    f.pool.get().unwrap().batch_execute("CREATE TRIGGER fail_minutes BEFORE INSERT ON logging_data WHEN (SELECT json_extract(tags,'$.metric') FROM statistics WHERE id=NEW.statistic_id)='study_minutes' BEGIN SELECT RAISE(ABORT,'injected private database failure'); END;").unwrap();
    let response = f
        .server
        .put(&path(DAY))
        .json(&json!({"cards_reviewed":42,"study_time_seconds":932}))
        .await;
    response.assert_status(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    assert!(!response.text().contains("injected"));
    assert_eq!(f.repo.find_all_metadata().unwrap().len(), 0);
    f.pool
        .get()
        .unwrap()
        .batch_execute("DROP TRIGGER fail_minutes;")
        .unwrap();
    put(&f, 10, 600).await;
    let (cards, minutes) = ids(&f);
    f.pool.get().unwrap().batch_execute("CREATE TRIGGER fail_update BEFORE UPDATE ON logging_data WHEN (SELECT json_extract(tags,'$.metric') FROM statistics WHERE id=NEW.statistic_id)='study_minutes' BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    f.server
        .put(&path(DAY))
        .json(&json!({"cards_reviewed":20,"study_time_seconds":1200}))
        .await
        .assert_status(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(value(&f, cards, DAY), 10);
    assert_eq!(value(&f, minutes, DAY), 10);
    f.pool
        .get()
        .unwrap()
        .batch_execute("DROP TRIGGER fail_update;")
        .unwrap();
    put(&f, 8, 480).await;
    assert_eq!(value(&f, cards, DAY), 8);
    assert_eq!(value(&f, minutes, DAY), 8);
}

#[tokio::test]
async fn rejects_invalid_boundary_data_before_writes() {
    let f = fixture();
    for (body, field) in [
        (json!({}), "cards_reviewed"),
        (json!({"cards_reviewed":1}), "study_time_seconds"),
        (
            json!({"cards_reviewed":-1,"study_time_seconds":0}),
            "cards_reviewed",
        ),
        (
            json!({"cards_reviewed":1.0,"study_time_seconds":0}),
            "cards_reviewed",
        ),
        (
            json!({"cards_reviewed":"1","study_time_seconds":0}),
            "cards_reviewed",
        ),
        (
            json!({"cards_reviewed":true,"study_time_seconds":0}),
            "cards_reviewed",
        ),
        (
            json!({"cards_reviewed":null,"study_time_seconds":0}),
            "cards_reviewed",
        ),
        (
            json!({"cards_reviewed":9223372036854775808_u64,"study_time_seconds":0}),
            "cards_reviewed",
        ),
        (
            json!({"cards_reviewed":1,"study_time_seconds":-1}),
            "study_time_seconds",
        ),
        (
            json!({"cards_reviewed":1,"study_time_seconds":9223372036854775808_u64}),
            "study_time_seconds",
        ),
    ] {
        let response = f.server.put(&path(DAY)).json(&body).await;
        response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(response.json::<Value>()["error"]["field"], field);
    }
    for body in [
        r#"{"cards_reviewed":1e400,"study_time_seconds":0}"#,
        r#"{"cards_reviewed":1,"study_time_seconds":1e400}"#,
        r#"{"cards_reviewed":0,"study_time_seconds":0,"extra":1}"#,
        r#"[]"#,
        r#"null"#,
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
        "2026-10-1",
        "26-10-21",
        "2026-10-21x",
        "+2026-10-21",
    ] {
        let response = f
            .server
            .put(&path(day))
            .json(&json!({"cards_reviewed":0,"study_time_seconds":0}))
            .await;
        response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(response.json::<Value>()["error"]["field"], "day");
    }
    f.server
        .put(&path(DAY))
        .text("{")
        .content_type("application/json")
        .await
        .assert_status(axum::http::StatusCode::BAD_REQUEST);
    f.server
        .put(&path(DAY))
        .text("{}")
        .content_type("text/plain")
        .await
        .assert_status(axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert!(f.repo.find_all_metadata().unwrap().is_empty());
    put(&f, i64::MAX, i64::MAX).await;
    let (cards, minutes) = ids(&f);
    assert_eq!(value(&f, cards, DAY), i64::MAX);
    assert_eq!(value(&f, minutes, DAY), i64::MAX / 60);
}

#[tokio::test]
async fn generated_openapi_describes_the_emitted_contract() {
    let f = fixture();
    let response = f.server.get("/openapi.json").await;
    response.assert_status_success();
    let doc: Value = response.json();
    let put = &doc["paths"]["/integrations/yomikata/days/{day}"]["put"];
    assert!(put.is_object());
    for status in ["204", "400", "409", "415", "422", "500"] {
        assert!(put["responses"][status].is_object(), "{status}");
    }
    let schema = &doc["components"]["schemas"]["YomikataDayRequest"];
    for field in ["cards_reviewed", "study_time_seconds"] {
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!(field)));
        assert_eq!(schema["properties"][field]["type"], "integer");
        assert_eq!(schema["properties"][field]["minimum"], 0);
        assert_eq!(schema["properties"][field]["maximum"], i64::MAX);
    }
}

#[tokio::test]
async fn aggregate_overflow_rolls_back_and_legacy_overflow_does_not_partly_adopt() {
    let f = fixture();
    put(&f, 10, 600).await;
    let (cards, minutes) = ids(&f);
    manual(&f, cards, i64::MAX, "overwrite").await;
    let response = f
        .server
        .put(&path(DAY))
        .json(&json!({"cards_reviewed":11,"study_time_seconds":660}))
        .await;
    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        response.json::<Value>()["error"]["code"],
        "projection_overflow"
    );
    assert_eq!(value(&f, cards, DAY), i64::MAX);
    assert_eq!(value(&f, minutes, DAY), 10);
    put(&f, 0, 0).await;
    assert_eq!(value(&f, cards, DAY), i64::MAX - 10);
    assert_eq!(value(&f, minutes, DAY), 0);

    let f = fixture();
    let original = legacy(&f, 2);
    f.repo
        .save_log_value(original[0], date(DAY), json!(10))
        .unwrap();
    f.repo
        .save_log_value(original[1], date("2020-01-01"), json!(i64::MAX))
        .unwrap();
    f.server
        .put(&path(DAY))
        .json(&json!({"cards_reviewed":8,"study_time_seconds":480}))
        .await
        .assert_status(axum::http::StatusCode::CONFLICT);
    assert_eq!(value(&f, original[0], DAY), 10);
    f.repo
        .save_log_value(original[0], date(DAY), json!(20))
        .unwrap();
    f.repo
        .save_log_value(original[1], date("2020-01-01"), json!(3))
        .unwrap();
    put(&f, 8, 480).await;
    assert_eq!(value(&f, original[0], DAY), 8);
    assert_eq!(value(&f, original[1], "2020-01-01"), 3);
}

#[tokio::test]
async fn seconds_are_durable_and_bound_identity_still_rejects_new_ambiguity() {
    #[derive(QueryableByName)]
    struct Seconds {
        #[diesel(sql_type=diesel::sql_types::BigInt)]
        study_time_seconds: i64,
    }
    let f = fixture();
    put(&f, 5, 61).await;
    let stored = diesel::sql_query(
        "SELECT study_time_seconds FROM yomikata_day_totals WHERE day='2026-10-21'",
    )
    .get_result::<Seconds>(&mut f.pool.get().unwrap())
    .unwrap();
    assert_eq!(stored.study_time_seconds, 61);
    let (cards, minutes) = ids(&f);
    let existing = f.repo.find_metadata_by_id(cards).unwrap();
    f.repo.create_metadata(existing).unwrap();
    f.server
        .put(&path(DAY))
        .json(&json!({"cards_reviewed":0,"study_time_seconds":0}))
        .await
        .assert_status(axum::http::StatusCode::CONFLICT);
    assert_eq!(value(&f, cards, DAY), 5);
    assert_eq!(value(&f, minutes, DAY), 1);
}

#[tokio::test]
async fn adoption_ignores_legacy_names_and_bundle_version_and_preserves_minute_goals() {
    use lifestats_core::statistic::{
        types::logable_statistics::integer_amount::IntegerAmountStatistic,
        value_objects::{goals::amount_goal::AmountGoal, statistic_metadata::StatisticMetadata},
    };
    let f = fixture();
    let stat = IntegerAmountStatistic::new(
        StatisticMetadata::new(
            StatisticId(0),
            "Custom category".into(),
            "Renamed legacy minutes".into(),
        )
        .with_tag("source".into(), "yomikata".into())
        .with_tag("metric".into(), "study_minutes".into())
        .with_tag("bundle_version".into(), "0".into()),
        Some(AmountGoal::new(30, true)),
        date("2020-01-01"),
    );
    let minutes = f
        .repo
        .create_metadata(AnyStatistic::IntegerAmount(stat))
        .unwrap()
        .get_metadata()
        .id();
    f.repo.save_log_value(minutes, date(DAY), json!(3)).unwrap();
    f.repo
        .save_goal(
            minutes,
            date(DAY),
            Some(json!({"target_amount":12,"is_at_least":true})),
        )
        .unwrap();
    let goal_before = f.repo.find_default_goal(minutes).unwrap();
    let daily_before = format!("{:?}", f.repo.find_goal(minutes, date(DAY)).unwrap());
    put(&f, 4, 240).await;
    assert_eq!(ids(&f).1, minutes);
    assert_eq!(value(&f, minutes, DAY), 4);
    assert_eq!(f.repo.find_default_goal(minutes).unwrap(), goal_before);
    assert_eq!(
        format!("{:?}", f.repo.find_goal(minutes, date(DAY)).unwrap()),
        daily_before
    );
    let stored = f.repo.find_metadata_by_id(minutes).unwrap();
    assert_eq!(stored.get_metadata().name(), "Renamed legacy minutes");
    assert_eq!(stored.get_default_goal_valid_since(), date("2020-01-01"));
}

#[tokio::test]
async fn zero_adoption_survives_reconstructed_service_and_server() {
    let f = fixture();
    put(&f, 0, 0).await;
    let (cards, minutes) = ids(&f);
    manual(&f, cards, 5, "sum").await;
    manual(&f, minutes, 2, "sum").await;
    let Fixture {
        server,
        repo: _,
        pool,
    } = f;
    drop(server);
    let repo = Arc::new(SqliteStatisticRepository::new(pool.clone()));
    let service = Arc::new(
        IntegrationService::new(
            repo.clone(),
            vec![Arc::new(YomikataBundleFactory)],
            None,
            None,
        )
        .with_yomikata_day_store(repo.clone()),
    );
    let state = AppState::new(
        repo.clone(),
        Arc::new(SqliteProgressRepository::new(pool.clone())),
        Arc::new(NoOpEventPublisher),
        Arc::new(PlottersRenderer::new()),
        service,
        Arc::new(SessionManager::new(repo.clone())),
    );
    let f = Fixture {
        server: TestServer::builder()
            .http_transport()
            .build(create_router(state))
            .unwrap(),
        repo,
        pool,
    };
    put(&f, 0, 0).await;
    assert_eq!(value(&f, cards, DAY), 5);
    assert_eq!(value(&f, minutes, DAY), 2);
    put(&f, 1, 60).await;
    assert_eq!(value(&f, cards, DAY), 6);
    assert_eq!(value(&f, minutes, DAY), 3);
}

#[tokio::test]
async fn regression_rejects_duplicate_fields_without_any_writes() {
    #[derive(QueryableByName)]
    struct Count {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        count: i64,
    }
    let f = fixture();
    for body in [
        r#"{"cards_reviewed":1,"cards_reviewed":2,"study_time_seconds":0}"#,
        r#"{"cards_reviewed":1,"study_time_seconds":0,"study_time_seconds":60}"#,
        r#"{"cards_reviewed":1,"cards_reviewed":1,"study_time_seconds":0}"#,
        r#"{"cards_reviewed":1,"study_time_seconds":0,"study_time_seconds":0}"#,
        r#"{"cards_reviewed":-1,"cards_reviewed":5,"study_time_seconds":0}"#,
        r#"{"cards_reviewed":5,"cards_reviewed":-1,"study_time_seconds":0}"#,
        r#"{"cards_reviewed":5,"study_time_seconds":"x","study_time_seconds":60}"#,
        r#"{"cards_reviewed":5,"study_time_seconds":60,"study_time_seconds":"x"}"#,
        r#"{"cards_reviewed":1,"cards_\u0072eviewed":2,"study_time_seconds":0}"#,
    ] {
        let response = f
            .server
            .put(&path(DAY))
            .text(body)
            .content_type("application/json")
            .await;
        response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(response.json::<Value>()["error"]["code"], "invalid_field");
        assert!(f.repo.find_all_metadata().unwrap().is_empty());
        for table in [
            "yomikata_day_totals",
            "yomikata_metric_bindings",
            "logging_data",
        ] {
            let rows = diesel::sql_query(format!("SELECT COUNT(*) AS count FROM {table}"))
                .get_result::<Count>(&mut f.pool.get().unwrap())
                .unwrap();
            assert_eq!(rows.count, 0, "{table} after rejecting {body}");
        }
    }
    put(&f, 5, 60).await;
    let (cards, minutes) = ids(&f);
    assert_eq!(value(&f, cards, DAY), 5);
    assert_eq!(value(&f, minutes, DAY), 1);
}

#[test]
fn regression_json_preserves_tagged_float_and_number_value_behavior() {
    use lifestats_application::dto::TrendDto;

    let trend =
        serde_json::from_str::<TrendDto>(r#"{"direction":"improving","strength":0.5}"#).unwrap();
    assert!(matches!(trend, TrendDto::Improving { strength } if strength == 0.5));
    assert_eq!(
        serde_json::from_str::<Value>("1.0").unwrap(),
        serde_json::from_str::<Value>("1.00").unwrap()
    );
}

#[tokio::test]
async fn regression_rejects_reserved_number_objects_without_any_writes() {
    #[derive(QueryableByName)]
    struct Count {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        count: i64,
    }
    let f = fixture();
    for (body, field) in [
        (
            r#"{"cards_reviewed":-0,"study_time_seconds":0}"#,
            "cards_reviewed",
        ),
        (
            r#"{"cards_reviewed":0,"study_time_seconds":-0}"#,
            "study_time_seconds",
        ),
        (
            r#"{"cards_reviewed":{"$serde_json::private::Number":"42"},"study_time_seconds":0}"#,
            "cards_reviewed",
        ),
        (
            r#"{"cards_reviewed":0,"study_time_seconds":{"$serde_json::private::Number":"42"}}"#,
            "study_time_seconds",
        ),
        (
            r#"{"cards_reviewed":999999999999999999999999999999999999999999999999999999,"study_time_seconds":0}"#,
            "cards_reviewed",
        ),
        (
            r#"{"cards_reviewed":0,"study_time_seconds":1e400}"#,
            "study_time_seconds",
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
        assert_eq!(error["error"]["code"], "invalid_field");
        assert_eq!(error["error"]["field"], field);
        assert!(f.repo.find_all_metadata().unwrap().is_empty());
        for table in [
            "yomikata_day_totals",
            "yomikata_metric_bindings",
            "logging_data",
        ] {
            let rows = diesel::sql_query(format!("SELECT COUNT(*) AS count FROM {table}"))
                .get_result::<Count>(&mut f.pool.get().unwrap())
                .unwrap();
            assert_eq!(rows.count, 0, "{table} after rejecting {field}");
        }
    }
}

#[tokio::test]
async fn regression_recreation_projects_only_the_requested_final_source_total() {
    for metric in ["cards_reviewed", "study_minutes"] {
        for new_source in [0, 3] {
            let f = fixture();
            put(&f, 10, 600).await;
            let earlier = "2026-10-20";
            f.server
                .put(&path(earlier))
                .json(&json!({"cards_reviewed":7,"study_time_seconds":420}))
                .await
                .assert_status(axum::http::StatusCode::NO_CONTENT);
            let (cards, minutes) = ids(&f);
            let deleted = if metric == "cards_reviewed" {
                cards
            } else {
                minutes
            };
            let metadata = f.repo.find_metadata_by_id(deleted).unwrap();
            f.repo.delete_metadata(deleted).unwrap();
            let replacement = f
                .repo
                .create_metadata(metadata)
                .unwrap()
                .get_metadata()
                .id();
            manual(&f, replacement, i64::MAX - new_source, "sum").await;
            f.repo
                .save_log_value(replacement, date(earlier), json!(2))
                .unwrap();
            let (new_cards, new_seconds) = if metric == "cards_reviewed" {
                (new_source, 600)
            } else {
                (10, new_source * 60)
            };
            put(&f, new_cards, new_seconds).await;
            assert_eq!(value(&f, replacement, DAY), i64::MAX);
            assert_eq!(value(&f, replacement, earlier), 9);
            assert_eq!(
                if metric == "cards_reviewed" {
                    ids(&f).0
                } else {
                    ids(&f).1
                },
                replacement
            );
            put(&f, new_cards, new_seconds).await;
            assert_eq!(value(&f, replacement, DAY), i64::MAX);
            assert_eq!(value(&f, replacement, earlier), 9);
            put(&f, 0, 0).await;
            assert_eq!(value(&f, replacement, DAY), i64::MAX - new_source);
        }
    }
}
