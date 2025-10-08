use std::collections::HashSet;

use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, Utc};

use crate::{
    error::ApplicationResult,
    integrations::tokens::{AccessToken, FitbitTokenData, RefreshToken},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FitbitMetric {
    SleepDuration,
    WakeUpTime,
    FellAsleepTime,
    SleepEfficiency,
    TotalDurationAwake,
    Steps,
    CaloriesOut,
    ActiveMinutes,
    CardioMinutes,
    PeakMinutes,
}

impl FitbitMetric {
    pub fn canonical_tag_value(&self) -> &'static str {
        match self {
            FitbitMetric::SleepDuration => "sleep_duration",
            FitbitMetric::WakeUpTime => "wake_up_time",
            FitbitMetric::FellAsleepTime => "fell_asleep_time",
            FitbitMetric::SleepEfficiency => "sleep_efficiency",
            FitbitMetric::TotalDurationAwake => "total_duration_awake",
            FitbitMetric::Steps => "steps",
            FitbitMetric::CaloriesOut => "calories_out",
            FitbitMetric::ActiveMinutes => "active_minutes",
            FitbitMetric::CardioMinutes => "cardio_minutes",
            FitbitMetric::PeakMinutes => "peak_minutes",
        }
    }

    pub fn required_scopes(&self) -> &'static [&'static str] {
        match self {
            FitbitMetric::SleepDuration
            | FitbitMetric::WakeUpTime
            | FitbitMetric::FellAsleepTime
            | FitbitMetric::SleepEfficiency
            | FitbitMetric::TotalDurationAwake => &["sleep"],
            FitbitMetric::Steps | FitbitMetric::CaloriesOut | FitbitMetric::ActiveMinutes => {
                &["activity"]
            }
            FitbitMetric::CardioMinutes | FitbitMetric::PeakMinutes => &["heartrate"],
        }
    }

    pub fn required_scope_set(&self) -> HashSet<&'static str> {
        self.required_scopes().iter().copied().collect()
    }
}

#[derive(Clone, Debug, Default)]
pub struct FitbitActivityData {
    pub steps: Option<u32>,
    pub calories_out: Option<u32>,
    pub active_minutes: Option<u32>,
    pub cardio_minutes: Option<u32>,
    pub peak_minutes: Option<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct FitbitSleepData {
    pub total_duration_asleep: Option<ChronoDuration>,
    pub total_duration_awake: Option<ChronoDuration>,
    pub fell_asleep_at_utc: Option<DateTime<Utc>>,
    pub woke_up_at_utc: Option<DateTime<Utc>>,
    pub sleep_efficiency: Option<u8>,
}

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
