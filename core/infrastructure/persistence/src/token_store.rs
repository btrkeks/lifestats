use std::sync::Arc;

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::SqliteConnection;
use lifestats_application::error::{ApplicationError, ApplicationResult};
use lifestats_application::integrations::fitbit::ports::TokenStorePort;
use lifestats_application::integrations::tokens::{AccessToken, FitbitTokenData, RefreshToken};

use crate::db::models::{IntegrationRow, IntegrationUpsert};
use crate::db::schema::integrations;

type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

pub struct SqliteTokenStore {
    pool: Arc<SqlitePool>,
}

impl SqliteTokenStore {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    fn connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<SqliteConnection>>, ApplicationError> {
        self.pool
            .get()
            .map_err(|err| ApplicationError::Infrastructure(err.into()))
    }

    fn canonicalize_source(source: &str) -> Option<String> {
        let canonical = source.trim().to_ascii_lowercase();
        if canonical.is_empty() {
            None
        } else {
            Some(format!("source={canonical}"))
        }
    }

    fn row_to_token(row: &IntegrationRow) -> ApplicationResult<Option<FitbitTokenData>> {
        let (Some(access), Some(refresh), Some(expires)) = (
            row.access_token.clone(),
            row.refresh_token.clone(),
            row.expires_at_epoch_seconds,
        ) else {
            return Ok(None);
        };

        let expires_at = DateTime::<Utc>::from_timestamp(expires, 0).ok_or_else(|| {
            ApplicationError::Infrastructure(anyhow!(
                "invalid expires_at_epoch_seconds ({expires}) for integration {}",
                row.source_name
            ))
        })?;

        let mut scopes: Vec<String> = row
            .scopes
            .clone()
            .unwrap_or_default()
            .split_whitespace()
            .map(|scope| scope.to_string())
            .collect();
        scopes.sort();
        scopes.dedup();

        Ok(Some(FitbitTokenData {
            access_token: AccessToken::new(access),
            refresh_token: RefreshToken::new(refresh),
            expires_at,
            scopes,
        }))
    }
}

#[async_trait]
impl TokenStorePort for SqliteTokenStore {
    async fn load(&self, source: &str) -> ApplicationResult<Option<FitbitTokenData>> {
        let mut conn = self.connection()?;
        let row = integrations::table
            .filter(integrations::source_name.eq(source))
            .first::<IntegrationRow>(&mut conn)
            .optional()
            .map_err(|err| ApplicationError::Infrastructure(err.into()))?;

        match row {
            None => Ok(None),
            Some(row) => Self::row_to_token(&row),
        }
    }

    async fn save(&self, source: &str, token: &FitbitTokenData) -> ApplicationResult<()> {
        let mut conn = self.connection()?;

        let mut scopes = token.scopes.clone();
        scopes.sort();
        scopes.dedup();
        let scopes_value = if scopes.is_empty() {
            None
        } else {
            Some(scopes.join(" "))
        };

        let record = IntegrationUpsert {
            source_name: source.to_string(),
            access_token: Some(token.access_token.as_str().to_string()),
            refresh_token: Some(token.refresh_token.as_str().to_string()),
            expires_at_epoch_seconds: Some(token.expires_at.timestamp()),
            scopes: scopes_value,
            last_sync_at: None,
            canonical_tags: Self::canonicalize_source(source),
        };

        let access_token = record.access_token.clone();
        let refresh_token = record.refresh_token.clone();
        let expires_at = record.expires_at_epoch_seconds;
        let scopes = record.scopes.clone();
        let canonical_tags = record.canonical_tags.clone();

        diesel::insert_into(integrations::table)
            .values(&record)
            .on_conflict(integrations::source_name)
            .do_update()
            .set((
                integrations::access_token.eq(access_token),
                integrations::refresh_token.eq(refresh_token),
                integrations::expires_at_epoch_seconds.eq(expires_at),
                integrations::scopes.eq(scopes),
                integrations::canonical_tags.eq(canonical_tags),
            ))
            .execute(&mut conn)
            .map_err(|err| ApplicationError::Infrastructure(err.into()))?;

        Ok(())
    }

    async fn set_last_sync_at(&self, source: &str, t: DateTime<Utc>) -> ApplicationResult<()> {
        let mut conn = self.connection()?;
        let timestamp = t.to_rfc3339();
        let canonical = Self::canonicalize_source(source);

        diesel::insert_into(integrations::table)
            .values((
                integrations::source_name.eq(source),
                integrations::last_sync_at.eq(Some(timestamp.clone())),
                integrations::canonical_tags.eq(canonical.clone()),
            ))
            .on_conflict(integrations::source_name)
            .do_update()
            .set((
                integrations::last_sync_at.eq(Some(timestamp)),
                integrations::canonical_tags.eq(canonical),
            ))
            .execute(&mut conn)
            .map_err(|err| ApplicationError::Infrastructure(err.into()))?;

        Ok(())
    }

    async fn last_sync_at(&self, source: &str) -> ApplicationResult<Option<DateTime<Utc>>> {
        let mut conn = self.connection()?;
        let row = integrations::table
            .filter(integrations::source_name.eq(source))
            .select(integrations::last_sync_at)
            .first::<Option<String>>(&mut conn)
            .optional()
            .map_err(|err| ApplicationError::Infrastructure(err.into()))?;

        match row {
            None => Ok(None),
            Some(None) => Ok(None),
            Some(Some(value)) => {
                let parsed = DateTime::parse_from_rfc3339(&value)
                    .with_context(|| format!("invalid last_sync_at stored for source {source}"))?
                    .with_timezone(&Utc);
                Ok(Some(parsed))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::MIGRATIONS;
    use chrono::{Duration, Utc};
    use diesel_migrations::MigrationHarness;

    fn setup_store() -> SqliteTokenStore {
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("failed to build test pool");

        {
            let mut conn = pool.get().expect("failed to get connection");
            conn.run_pending_migrations(MIGRATIONS)
                .expect("failed to run migrations");
        }

        SqliteTokenStore::new(Arc::new(pool))
    }

    fn sample_token() -> FitbitTokenData {
        FitbitTokenData {
            access_token: AccessToken::new("access"),
            refresh_token: RefreshToken::new("refresh"),
            expires_at: Utc::now() + Duration::minutes(30),
            scopes: vec!["sleep".to_string(), "activity".to_string()],
        }
    }

    #[tokio::test]
    async fn save_and_load_roundtrip() {
        let store = setup_store();
        let token = sample_token();

        store.save("fitbit", &token).await.expect("save token");

        let loaded = store
            .load("fitbit")
            .await
            .expect("load token")
            .expect("token present");

        assert_eq!(
            loaded.scopes,
            vec!["activity".to_string(), "sleep".to_string()]
        );
        assert_eq!(loaded.access_token.as_str(), token.access_token.as_str());
        assert_eq!(loaded.refresh_token.as_str(), token.refresh_token.as_str());
    }

    #[tokio::test]
    async fn updates_last_sync_timestamp() {
        let store = setup_store();
        let now = Utc::now();

        store
            .set_last_sync_at("fitbit", now)
            .await
            .expect("set last sync");

        let retrieved = store
            .last_sync_at("fitbit")
            .await
            .expect("load last sync")
            .expect("value present");

        assert!((retrieved - now).num_seconds().abs() < 2);
    }
}
