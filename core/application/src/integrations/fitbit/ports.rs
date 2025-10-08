use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};

use crate::{
    error::ApplicationResult,
    integrations::tokens::{AccessToken, FitbitTokenData, RefreshToken},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FitbitMetric(pub String);

#[derive(Clone, Debug)]
pub struct FitbitActivityData;

#[derive(Clone, Debug)]
pub struct FitbitSleepData;

#[async_trait]
pub trait FitbitOAuthApi: Send + Sync {
    async fn exchange_code_for_token(
        &self,
        code: &str,
        pkce_verifier: Option<String>,
    ) -> ApplicationResult<FitbitTokenData>;

    async fn refresh(&self, refresh_token: &RefreshToken) -> ApplicationResult<FitbitTokenData>;
}

#[async_trait]
pub trait FitbitDataApi: Send + Sync {
    async fn get_activity_data(
        &self,
        access_token: &AccessToken,
        date: NaiveDate,
    ) -> ApplicationResult<FitbitActivityData>;

    async fn get_sleep_data(
        &self,
        access_token: &AccessToken,
        date: NaiveDate,
    ) -> ApplicationResult<FitbitSleepData>;
}

#[async_trait]
pub trait TokenStorePort: Send + Sync {
    async fn load(&self, source: &str) -> ApplicationResult<Option<FitbitTokenData>>;
    async fn save(&self, source: &str, token: &FitbitTokenData) -> ApplicationResult<()>;
    async fn set_last_sync_at(&self, source: &str, t: DateTime<Utc>) -> ApplicationResult<()>;
    async fn last_sync_at(&self, source: &str) -> ApplicationResult<Option<DateTime<Utc>>>;
}

pub trait Clock: Send + Sync {
    fn now_utc(&self) -> DateTime<Utc>;
}
