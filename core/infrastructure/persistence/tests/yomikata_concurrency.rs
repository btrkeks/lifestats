use chrono::NaiveDate;
use diesel::{
    connection::SimpleConnection,
    r2d2::{ConnectionManager, CustomizeConnection, Pool},
    SqliteConnection,
};
use diesel_migrations::MigrationHarness;
use events::event_port::{Event, EventPublisher};
use lifestats_application::{
    cqrs::CommandHandler,
    use_cases::{
        commands::log_value::{LogValueCommand, LogValueCommandHandler, LogValuePayload},
        integration::YomikataBundleFactory,
    },
};
use lifestats_core::{
    statistic::{
        ports::statistic_repository::StatisticRepository,
        value_objects::update_policies::amount::AmountUpdatePolicy,
    },
    yomikata::{YomikataDayStore, YomikataDayTotals},
};
use lifestats_persistence::{pool::MIGRATIONS, repository::SqliteStatisticRepository};
use std::{
    collections::HashMap,
    sync::{Arc, Barrier},
    time::{SystemTime, UNIX_EPOCH},
};

struct NoEvents;
impl EventPublisher for NoEvents {
    fn publish_event_async(&self, _: Event) {}
    fn publish_event_sync(&self, _: Event) -> anyhow::Result<()> {
        Ok(())
    }
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
#[test]
fn concurrent_generic_commands_preserve_every_manual_addition_and_the_latest_source_total() {
    let path = std::env::temp_dir().join(format!(
        "lifestats-yomikata-race-{}-{}.db",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let manager = ConnectionManager::<SqliteConnection>::new(path.to_string_lossy());
    let pool = Arc::new(
        Pool::builder()
            .max_size(6)
            .connection_customizer(Box::new(Customize))
            .build(manager)
            .unwrap(),
    );
    pool.get()
        .unwrap()
        .run_pending_migrations(MIGRATIONS)
        .unwrap();
    let repo = Arc::new(SqliteStatisticRepository::new(pool.clone()));
    let day: NaiveDate = "2026-10-21".parse().unwrap();
    repo.replace_day(
        day,
        YomikataDayTotals::default(),
        YomikataBundleFactory::definitions(),
    )
    .unwrap();
    let cards = repo
        .find_by_tags(&HashMap::from([
            ("source".into(), "yomikata".into()),
            ("metric".into(), "cards_reviewed".into()),
        ]))
        .unwrap()
        .remove(0);
    let id = cards.get_metadata().id();
    let barrier = Arc::new(Barrier::new(5));
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let repo = repo.clone();
            let barrier = barrier.clone();
            let cards = cards.clone();
            scope.spawn(move || {
                let handler = LogValueCommandHandler::new(repo, Arc::new(NoEvents));
                barrier.wait();
                for _ in 0..100 {
                    handler
                        .handle(LogValueCommand {
                            statistic_id: id,
                            date: day,
                            statistic: cards.clone(),
                            payload: LogValuePayload::Integer {
                                value: 1,
                                update_policy: AmountUpdatePolicy::Sum,
                            },
                        })
                        .unwrap();
                }
            });
        }
        let repo = repo.clone();
        let barrier = barrier.clone();
        scope.spawn(move || {
            barrier.wait();
            for total in 1..=100 {
                repo.replace_day(
                    day,
                    YomikataDayTotals::new(total, total * 61).unwrap(),
                    YomikataBundleFactory::definitions(),
                )
                .unwrap();
            }
        });
    });
    assert_eq!(
        repo.find_log_value(id, day).unwrap(),
        Some(serde_json::json!(500))
    );
    repo.replace_day(
        day,
        YomikataDayTotals::default(),
        YomikataBundleFactory::definitions(),
    )
    .unwrap();
    assert_eq!(
        repo.find_log_value(id, day).unwrap(),
        Some(serde_json::json!(400))
    );
    drop(repo);
    drop(pool);
    std::fs::remove_file(path).unwrap();
}
