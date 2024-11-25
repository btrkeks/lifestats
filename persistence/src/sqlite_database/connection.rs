use std::env;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::path::PathBuf;
use anyhow::{Context, Result};
use dotenvy::dotenv;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Establishes a connection to the SQLite database
pub fn establish_connection() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut conn = SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    // Run migrations
    conn.run_pending_migrations(MIGRATIONS)
        .unwrap_or_else(|e| panic!("Error running migrations: {}", e));

    conn
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use diesel::connection::SimpleConnection;

    /// Creates a connection for testing purposes
    pub fn establish_test_connection() -> Result<SqliteConnection> {
        let mut conn = SqliteConnection::establish(":memory:")?;
        conn.batch_execute("PRAGMA foreign_keys = ON;")?;
        conn.run_pending_migrations(MIGRATIONS)
            .map_err(|e| anyhow::anyhow!("Error running migrations: {}", e))?;
        Ok(conn)
    }

    #[test]
    fn test_database_connection() {
        let conn = establish_test_connection();
        assert!(conn.is_ok(), "Should be able to establish database connection");
    }
}