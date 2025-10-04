#![cfg(feature = "db-compat-test")]

use anyhow::{Context, Result};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use lifestats_domain::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_persistence::repository::SqliteStatisticRepository;
use std::env;
use std::sync::Arc;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

/// Creates a connection pool to the database file specified by the
/// `COMPAT_DB_URL` environment variable.
fn get_compat_db_pool() -> Result<Pool<ConnectionManager<SqliteConnection>>> {
    let db_url = env::var("COMPAT_DB_URL")
        .context("COMPAT_DB_URL environment variable not set. Please point it to the test database snapshot.")?;

    let manager = ConnectionManager::<SqliteConnection>::new(&db_url);
    let pool = Pool::builder()
        .max_size(1)
        .build(manager)
        .context(format!("Failed to create pool for database at {}", db_url))?;

    Ok(pool)
}

#[test]
fn test_can_load_all_statistics_from_snapshot() -> Result<()> {
    // --- 1. SETUP ---
    // This test will fail if the env var isn't set, which is intentional.
    let pool = get_compat_db_pool()?;

    // --- 2. RUN MIGRATIONS ---
    // Apply any pending migrations to the snapshot database to ensure it has
    // the latest schema (including the default_goal_valid_since column).
    {
        let mut conn = pool
            .get()
            .context("Failed to get connection for migrations")?;
        conn.run_pending_migrations(MIGRATIONS).map_err(|e| {
            anyhow::anyhow!("Failed to run migrations on the snapshot database: {}", e)
        })?;
    }

    let repo = SqliteStatisticRepository::new(Arc::new(pool));

    // --- 3. ACT ---
    // The core of the test: attempt the read-only operation that failed before.
    let result = repo.find_all_metadata();

    // --- 4. ASSERT ---
    // We only assert that the operation succeeded. We don't care about the
    // content, which makes the test robust against daily data changes.
    assert!(
        result.is_ok(),
        "Failed to load and parse all statistics from the database snapshot. Error: {:?}",
        result.err()
    );

    // Optional: If it succeeds, you can add a simple sanity check.
    if let Ok(stats) = result {
        println!(
            "Successfully loaded {} statistics from the snapshot.",
            stats.len()
        );
        assert!(
            !stats.is_empty(),
            "Expected the database snapshot to contain at least one statistic."
        );
    }

    Ok(())
}
