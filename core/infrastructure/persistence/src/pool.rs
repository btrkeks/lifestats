use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, CustomizeConnection, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use dotenvy::dotenv;
use once_cell::sync::Lazy;
use std::env;
use std::sync::Arc;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

static DATABASE_POOL: Lazy<Arc<Pool<ConnectionManager<SqliteConnection>>>> = Lazy::new(|| {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = create_pool_from_url(&database_url);
    Arc::new(pool)
});

#[derive(Debug)]
struct SQLiteConnectionCustomizer;

impl CustomizeConnection<SqliteConnection, diesel::r2d2::Error> for SQLiteConnectionCustomizer {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        conn.batch_execute(
            "
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 30000;
        PRAGMA journal_mode = WAL;
        ",
        )
        .map_err(diesel::r2d2::Error::QueryError)
    }
}

fn create_pool_from_url(database_url: &str) -> Pool<ConnectionManager<SqliteConnection>> {
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);

    let pool = Pool::builder()
        .test_on_check_out(true)
        .connection_timeout(std::time::Duration::from_secs(30))
        .max_size(10)
        .connection_customizer(Box::new(SQLiteConnectionCustomizer))
        .build(manager)
        .expect("Failed to create database pool");

    let mut conn = pool
        .get()
        .expect("Failed to get SQLite connection for running migrations");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run embedded SQLite migrations");

    pool
}

pub fn get_database_pool() -> Arc<Pool<ConnectionManager<SqliteConnection>>> {
    Arc::clone(&*DATABASE_POOL)
}

pub fn get_test_pool() -> Pool<ConnectionManager<SqliteConnection>> {
    create_pool_from_url(":memory:")
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::sql_query;
    use diesel::sql_types::BigInt;
    use diesel::QueryableByName;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(QueryableByName)]
    struct CountRow {
        #[diesel(sql_type = BigInt)]
        count: i64,
    }

    #[test]
    fn test_foreign_keys_enabled() {
        let pool = get_test_pool();
        let mut conn1 = pool.get().expect("Failed to get connection");
        let mut conn2 = pool.get().expect("Failed to get connection");
        let mut conn3 = pool.get().expect("Failed to get connection");

        test_connection_enables_foreign_keys(&mut conn1);
        test_connection_enables_foreign_keys(&mut conn2);
        test_connection_enables_foreign_keys(&mut conn3);
    }

    fn test_connection_enables_foreign_keys(conn: &mut SqliteConnection) {
        // Then verify actual constraint enforcement
        // Create parent and child tables
        sql_query(
            r#"
            CREATE TABLE parent (
                id INTEGER PRIMARY KEY
            )
        "#,
        )
        .execute(conn)
        .expect("Failed to create parent table");

        sql_query(
            r#"
            CREATE TABLE child (
                id INTEGER PRIMARY KEY,
                parent_id INTEGER,
                FOREIGN KEY (parent_id) REFERENCES parent(id)
            )
        "#,
        )
        .execute(conn)
        .expect("Failed to create child table");

        // Insert into parent - this should work
        sql_query("INSERT INTO parent (id) VALUES (1)")
            .execute(conn)
            .expect("Failed to insert into parent");

        // Try to insert into child with non-existent parent_id
        // This should fail if foreign keys are enabled
        let result = sql_query("INSERT INTO child (id, parent_id) VALUES (1, 999)").execute(conn);

        assert!(result.is_err(), "Foreign key constraint was not enforced!");

        // Verify we can insert with valid foreign key
        let result = sql_query("INSERT INTO child (id, parent_id) VALUES (1, 1)").execute(conn);

        assert!(result.is_ok(), "Failed to insert with valid foreign key");
    }

    #[test]
    fn create_pool_from_url_applies_pending_migrations() {
        let db_path = std::env::temp_dir().join(format!(
            "lifestats-pool-test-{}.db",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time before unix epoch")
                .as_nanos()
        ));
        let database_url = db_path.to_string_lossy().to_string();

        let pool = create_pool_from_url(&database_url);
        let mut conn = pool.get().expect("Failed to get connection");

        let existing_tables = sql_query(
            "SELECT COUNT(*) as count FROM sqlite_master WHERE type = 'table' AND name = 'ebook_sync_events'",
        )
        .get_result::<CountRow>(&mut conn)
        .expect("Failed to query sqlite_master");

        std::fs::remove_file(&db_path).expect("failed to remove temp database");

        assert_eq!(
            existing_tables.count, 1,
            "expected embedded migrations to create ebook_sync_events"
        );
    }
}
