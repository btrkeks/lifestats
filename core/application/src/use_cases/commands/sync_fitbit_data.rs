use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration as StdDuration;

use anyhow::anyhow;
use chrono::{Duration as ChronoDuration, NaiveDate, Timelike};
use chrono_tz::Tz;
use futures::stream::{self, TryStreamExt};
use rand::Rng;
use serde_json::json;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{instrument, Span};
use uuid::Uuid;

use crate::cqrs::Command;
use crate::error::{ApplicationError, ApplicationResult};
use crate::integrations::fitbit::error::FitbitUpstreamError;
use crate::integrations::fitbit::{
    Clock, FitbitActivityData, FitbitDataApi, FitbitMetric, FitbitOAuthApi, FitbitSleepData,
    TokenStorePort,
};
use crate::shared::tags::TagSet;
use crate::statistic::ports::TagQueryableStatisticRepository;
use crate::StatisticRepository;

const MAX_SYNC_RANGE_DAYS: i64 = 31;
const DEFAULT_TOKEN_EXPIRY_SKEW: StdDuration = StdDuration::from_secs(300);
const DEFAULT_MAX_CONCURRENCY: usize = 3;
const DEFAULT_MAX_RETRIES: usize = 3;
const DEFAULT_BASE_BACKOFF_MS: u64 = 500;
const DEFAULT_BACKOFF_MULTIPLIER: f64 = 2.0;
const DEFAULT_HTTP_TIMEOUT: StdDuration = StdDuration::from_secs(15);
const FITBIT_SOURCE: &str = "fitbit";

#[derive(Clone, Copy, Debug)]
pub struct SyncFitbitDataCommand {
    pub since: NaiveDate,
    pub until: NaiveDate,
}

impl Command for SyncFitbitDataCommand {
    type Result = ();
}

pub struct SyncFitbitDataCommandHandler {
    oauth_api: Arc<dyn FitbitOAuthApi>,
    data_api: Arc<dyn FitbitDataApi>,
    token_store: Arc<dyn TokenStorePort>,
    tag_repository: Arc<dyn TagQueryableStatisticRepository>,
    statistic_repository: Arc<dyn StatisticRepository + Send + Sync>,
    clock: Arc<dyn Clock>,
    timezone: chrono_tz::Tz,
    token_expiry_skew: StdDuration,
    max_concurrency: usize,
    max_retries: usize,
    base_backoff_ms: u64,
    backoff_multiplier: f64,
    http_timeout: StdDuration,
    enabled_metrics: HashSet<FitbitMetric>,
}

impl SyncFitbitDataCommandHandler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        oauth_api: Arc<dyn FitbitOAuthApi>,
        data_api: Arc<dyn FitbitDataApi>,
        token_store: Arc<dyn TokenStorePort>,
        tag_repository: Arc<dyn TagQueryableStatisticRepository>,
        statistic_repository: Arc<dyn StatisticRepository + Send + Sync>,
        clock: Arc<dyn Clock>,
        timezone: chrono_tz::Tz,
        enabled_metrics: HashSet<FitbitMetric>,
    ) -> Self {
        Self {
            oauth_api,
            data_api,
            token_store,
            tag_repository,
            statistic_repository,
            clock,
            timezone,
            token_expiry_skew: DEFAULT_TOKEN_EXPIRY_SKEW,
            max_concurrency: DEFAULT_MAX_CONCURRENCY,
            max_retries: DEFAULT_MAX_RETRIES,
            base_backoff_ms: DEFAULT_BASE_BACKOFF_MS,
            backoff_multiplier: DEFAULT_BACKOFF_MULTIPLIER,
            http_timeout: DEFAULT_HTTP_TIMEOUT,
            enabled_metrics,
        }
    }

    pub fn with_token_expiry_skew(mut self, skew: StdDuration) -> Self {
        self.token_expiry_skew = skew;
        self
    }

    pub fn with_max_concurrency(mut self, max_concurrency: usize) -> Self {
        self.max_concurrency = max_concurrency.max(1);
        self
    }

    pub fn with_max_retries(mut self, max_retries: usize) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn with_backoff(mut self, base_ms: u64, multiplier: f64) -> Self {
        self.base_backoff_ms = base_ms.max(1);
        self.backoff_multiplier = multiplier.max(1.0);
        self
    }

    pub fn with_http_timeout(mut self, timeout: StdDuration) -> Self {
        self.http_timeout = timeout;
        self
    }

    #[instrument(skip_all, fields(run_id = tracing::field::Empty))]
    pub async fn handle(&self, command: SyncFitbitDataCommand) -> ApplicationResult<()> {
        let sync_run_id = Uuid::new_v4();
        Span::current().record("run_id", sync_run_id.to_string().as_str());
        self.validate_range(&command)?;

        let enabled_metrics: Vec<FitbitMetric> = self.enabled_metrics.iter().cloned().collect();
        if enabled_metrics.is_empty() {
            return Err(ApplicationError::InvalidInput(
                "No Fitbit metrics enabled for synchronization".into(),
            ));
        }

        let token = self.load_token(sync_run_id).await?;
        let token_state = Arc::new(Mutex::new(token));

        let metric_to_statistic = self.map_metrics_to_statistics(&enabled_metrics).await?;
        let metric_map = Arc::new(metric_to_statistic);

        let cache = Arc::new(Mutex::new(HashMap::new()));

        let mut dates = Vec::new();
        let mut current = command.since;
        while current <= command.until {
            dates.push(current);
            current = current
                .succ_opt()
                .ok_or_else(|| ApplicationError::InvalidInput("Date overflow".into()))?;
        }

        stream::iter(dates.into_iter().map(Ok::<_, ApplicationError>))
            .try_for_each_concurrent(self.max_concurrency, |date| {
                let handler = self;
                let enabled_metrics = enabled_metrics.clone();
                let metric_map = Arc::clone(&metric_map);
                let cache = Arc::clone(&cache);
                let token_state = Arc::clone(&token_state);

                async move {
                    handler
                        .process_date(
                            date,
                            &enabled_metrics,
                            metric_map,
                            cache,
                            token_state,
                            sync_run_id,
                        )
                        .await
                }
            })
            .await?;

        self.token_store
            .set_last_sync_at(FITBIT_SOURCE, self.clock.now_utc())
            .await?;

        Ok(())
    }

    fn validate_range(&self, command: &SyncFitbitDataCommand) -> ApplicationResult<()> {
        if command.since > command.until {
            return Err(ApplicationError::InvalidInput(
                "`since` must be on or before `until`".into(),
            ));
        }

        let range = command.until.signed_duration_since(command.since);
        if range.num_days() > MAX_SYNC_RANGE_DAYS {
            return Err(ApplicationError::InvalidInput(format!(
                "Sync range too large: {} days (maximum {MAX_SYNC_RANGE_DAYS})",
                range.num_days()
            )));
        }

        Ok(())
    }

    async fn load_token(
        &self,
        sync_run_id: Uuid,
    ) -> ApplicationResult<crate::integrations::tokens::FitbitTokenData> {
        let mut token =
            self.token_store.load(FITBIT_SOURCE).await?.ok_or_else(|| {
                ApplicationError::InvalidInput("No Fitbit token configured".into())
            })?;

        self.ensure_scopes(&token)?;

        if self.is_token_expiring(&token) {
            token = self.refresh_token(&token, sync_run_id).await?;
            self.token_store.save(FITBIT_SOURCE, &token).await?;
        }

        Ok(token)
    }

    fn ensure_scopes(
        &self,
        token: &crate::integrations::tokens::FitbitTokenData,
    ) -> ApplicationResult<()> {
        let mut required: HashSet<String> = HashSet::new();
        for metric in &self.enabled_metrics {
            for scope in metric.required_scopes() {
                required.insert(scope.to_string());
            }
        }

        let provided: HashSet<String> = token
            .scopes
            .iter()
            .map(|scope| scope.to_ascii_lowercase())
            .collect();

        let missing: Vec<String> = required
            .into_iter()
            .filter(|scope| !provided.contains(scope))
            .collect();

        if missing.is_empty() {
            Ok(())
        } else {
            Err(ApplicationError::InvalidInput(format!(
                "Fitbit token missing required scopes: {}",
                missing.join(", ")
            )))
        }
    }

    fn is_token_expiring(&self, token: &crate::integrations::tokens::FitbitTokenData) -> bool {
        let skew = ChronoDuration::from_std(self.token_expiry_skew)
            .unwrap_or_else(|_| ChronoDuration::seconds(0));
        self.clock.now_utc() >= token.expires_at - skew
    }

    async fn refresh_token(
        &self,
        token: &crate::integrations::tokens::FitbitTokenData,
        _sync_run_id: Uuid,
    ) -> ApplicationResult<crate::integrations::tokens::FitbitTokenData> {
        let refreshed = self.oauth_api.refresh(&token.refresh_token).await?;
        Ok(refreshed)
    }

    async fn map_metrics_to_statistics(
        &self,
        metrics: &[FitbitMetric],
    ) -> ApplicationResult<HashMap<FitbitMetric, lifestats_core::shared::ids::StatisticId>> {
        let mut mapping = HashMap::new();
        for metric in metrics {
            let tags = TagSet::new([
                ("source", "fitbit"),
                ("metric", metric.canonical_tag_value()),
            ]);

            let statistic_id = self
                .tag_repository
                .find_statistic_id_by_exact_tags(&tags)
                .await?;

            match statistic_id {
                Some(id) => {
                    if mapping.insert(metric.clone(), id).is_some() {
                        return Err(ApplicationError::InvalidInput(format!(
                            "Duplicate mapping for metric {}",
                            metric.canonical_tag_value()
                        )));
                    }
                }
                None => {
                    return Err(ApplicationError::InvalidInput(format!(
                        "No statistic found for metric {}",
                        metric.canonical_tag_value()
                    )));
                }
            }
        }

        Ok(mapping)
    }

    async fn process_date(
        &self,
        date: NaiveDate,
        metrics: &[FitbitMetric],
        metric_map: Arc<HashMap<FitbitMetric, lifestats_core::shared::ids::StatisticId>>,
        cache: Arc<Mutex<FitbitCache>>,
        token_state: Arc<Mutex<crate::integrations::tokens::FitbitTokenData>>,
        sync_run_id: Uuid,
    ) -> ApplicationResult<()> {
        for metric in metrics {
            let statistic_id = metric_map.get(metric).ok_or_else(|| {
                ApplicationError::InvalidInput("Missing statistic mapping".into())
            })?;

            let data = self
                .fetch_metric_data(
                    date,
                    metric.clone(),
                    Arc::clone(&cache),
                    Arc::clone(&token_state),
                    sync_run_id,
                )
                .await?;

            if let Some((log_date, value)) = self.transform_metric(metric, date, &data)? {
                self.statistic_repository
                    .save_log_value(*statistic_id, log_date, value)?;
            }
        }

        Ok(())
    }

    async fn fetch_metric_data(
        &self,
        date: NaiveDate,
        metric: FitbitMetric,
        cache: Arc<Mutex<FitbitCache>>,
        token_state: Arc<Mutex<crate::integrations::tokens::FitbitTokenData>>,
        sync_run_id: Uuid,
    ) -> ApplicationResult<FitbitFetchedData> {
        if let Some(data) = self.try_cache(&cache, &metric, date).await {
            return Ok(data);
        }

        let data = self
            .fetch_with_retry(date, metric.clone(), Arc::clone(&token_state), sync_run_id)
            .await?;

        self.populate_cache(cache, date, &metric, data.clone())
            .await;

        Ok(data)
    }

    async fn try_cache(
        &self,
        cache: &Mutex<FitbitCache>,
        metric: &FitbitMetric,
        date: NaiveDate,
    ) -> Option<FitbitFetchedData> {
        let lock = cache.lock().await;
        lock.get(&(metric.clone(), date)).cloned()
    }

    async fn populate_cache(
        &self,
        cache: Arc<Mutex<FitbitCache>>,
        date: NaiveDate,
        metric: &FitbitMetric,
        data: FitbitFetchedData,
    ) {
        let related_metrics = self.metrics_for_category(metric);
        let mut lock = cache.lock().await;
        for related in related_metrics {
            lock.insert((related, date), data.clone());
        }
    }

    fn metrics_for_category(&self, metric: &FitbitMetric) -> Vec<FitbitMetric> {
        match metric_category(metric) {
            FitbitDataCategory::Sleep => self
                .enabled_metrics
                .iter()
                .filter(|m| matches!(metric_category(m), FitbitDataCategory::Sleep))
                .cloned()
                .collect(),
            FitbitDataCategory::Activity => self
                .enabled_metrics
                .iter()
                .filter(|m| matches!(metric_category(m), FitbitDataCategory::Activity))
                .cloned()
                .collect(),
        }
    }

    async fn fetch_with_retry(
        &self,
        date: NaiveDate,
        metric: FitbitMetric,
        token_state: Arc<Mutex<crate::integrations::tokens::FitbitTokenData>>,
        sync_run_id: Uuid,
    ) -> ApplicationResult<FitbitFetchedData> {
        let mut attempt = 0;
        let mut unauthorized_refreshed = false;

        loop {
            attempt += 1;

            let token_snapshot = {
                let guard = token_state.lock().await;
                guard.clone()
            };

            let fetch_result = self.execute_fetch(&metric, date, &token_snapshot).await;

            match fetch_result {
                Ok(data) => return Ok(data),
                Err(error) => match &error {
                    ApplicationError::FitbitSyncFailed(FitbitUpstreamError::Unauthorized)
                        if !unauthorized_refreshed =>
                    {
                        let refreshed = self.refresh_token(&token_snapshot, sync_run_id).await?;
                        self.token_store.save(FITBIT_SOURCE, &refreshed).await?;
                        let mut guard = token_state.lock().await;
                        *guard = refreshed;
                        unauthorized_refreshed = true;
                        continue;
                    }
                    ApplicationError::FitbitSyncFailed(FitbitUpstreamError::RateLimited {
                        retry_after,
                    }) => {
                        if attempt > self.max_retries {
                            return Err(error);
                        }

                        let backoff = retry_after
                            .as_ref()
                            .copied()
                            .unwrap_or_else(|| self.compute_backoff(attempt));
                        sleep(backoff).await;
                        continue;
                    }
                    _ => {
                        if attempt > self.max_retries {
                            return Err(error);
                        }

                        sleep(self.compute_backoff(attempt)).await;
                        continue;
                    }
                },
            }
        }
    }

    fn compute_backoff(&self, attempt: usize) -> StdDuration {
        let exponent = (attempt.saturating_sub(1)) as u32;
        let base = (self.base_backoff_ms as f64 * self.backoff_multiplier.powi(exponent as i32))
            .max(self.base_backoff_ms as f64);
        let mut rng = rand::rng();
        let jitter = rng.random_range(0.0..=base);
        StdDuration::from_millis(jitter as u64)
    }

    async fn execute_fetch(
        &self,
        metric: &FitbitMetric,
        date: NaiveDate,
        token: &crate::integrations::tokens::FitbitTokenData,
    ) -> ApplicationResult<FitbitFetchedData> {
        match metric_category(metric) {
            FitbitDataCategory::Sleep => {
                let api = Arc::clone(&self.data_api);
                let access_token = token.access_token.clone();

                match tokio::time::timeout(self.http_timeout, async move {
                    api.get_sleep_data(&access_token, date).await
                })
                .await
                {
                    Ok(Ok(data)) => Ok(FitbitFetchedData::Sleep(data)),
                    Ok(Err(error)) => Err(error),
                    Err(_) => Err(ApplicationError::FitbitSyncFailed(
                        FitbitUpstreamError::UpstreamServiceError(anyhow!(
                            "Fitbit request timed out"
                        )),
                    )),
                }
            }
            FitbitDataCategory::Activity => {
                let api = Arc::clone(&self.data_api);
                let access_token = token.access_token.clone();

                match tokio::time::timeout(self.http_timeout, async move {
                    api.get_activity_data(&access_token, date).await
                })
                .await
                {
                    Ok(Ok(data)) => Ok(FitbitFetchedData::Activity(data)),
                    Ok(Err(error)) => Err(error),
                    Err(_) => Err(ApplicationError::FitbitSyncFailed(
                        FitbitUpstreamError::UpstreamServiceError(anyhow!(
                            "Fitbit request timed out"
                        )),
                    )),
                }
            }
        }
    }

    fn transform_metric(
        &self,
        metric: &FitbitMetric,
        requested_date: NaiveDate,
        data: &FitbitFetchedData,
    ) -> ApplicationResult<Option<(NaiveDate, serde_json::Value)>> {
        match (metric_category(metric), data) {
            (FitbitDataCategory::Sleep, FitbitFetchedData::Sleep(sleep)) => {
                self.transform_sleep_metric(metric, requested_date, sleep)
            }
            (FitbitDataCategory::Activity, FitbitFetchedData::Activity(activity)) => {
                Ok(self.transform_activity_metric(metric, requested_date, activity))
            }
            _ => Err(ApplicationError::Infrastructure(anyhow!(
                "Mismatched Fitbit data category"
            ))),
        }
    }

    fn transform_sleep_metric(
        &self,
        metric: &FitbitMetric,
        requested_date: NaiveDate,
        sleep: &FitbitSleepData,
    ) -> ApplicationResult<Option<(NaiveDate, serde_json::Value)>> {
        let result = match metric {
            FitbitMetric::SleepDuration => sleep
                .total_duration_asleep
                .map(|duration| (requested_date, json!(duration.num_seconds()))),
            FitbitMetric::SleepEfficiency => sleep
                .sleep_efficiency
                .map(|efficiency| (requested_date, json!(efficiency))),
            FitbitMetric::WakeUpTime => sleep.woke_up_at_local.map(|time| {
                // Fitbit returns times in user's local timezone, use directly
                (
                    time.date(),
                    json!(format!(
                        "{:02}:{:02}:{:02}",
                        time.hour(),
                        time.minute(),
                        time.second()
                    )),
                )
            }),
            FitbitMetric::FellAsleepTime => sleep.fell_asleep_at_local.map(|time| {
                // Fitbit returns times in user's local timezone, use directly
                (
                    time.date(),
                    json!(format!(
                        "{:02}:{:02}:{:02}",
                        time.hour(),
                        time.minute(),
                        time.second()
                    )),
                )
            }),
            FitbitMetric::TotalDurationAwake => sleep
                .total_duration_awake
                .map(|duration| (requested_date, json!(duration.num_seconds()))),
            _ => None,
        };

        Ok(result)
    }

    fn transform_activity_metric(
        &self,
        metric: &FitbitMetric,
        date: NaiveDate,
        activity: &FitbitActivityData,
    ) -> Option<(NaiveDate, serde_json::Value)> {
        match metric {
            FitbitMetric::Steps => activity.steps.map(|steps| (date, json!(steps))),
            FitbitMetric::CaloriesOut => activity
                .calories_out
                .map(|calories| (date, json!(calories))),
            FitbitMetric::ActiveMinutes => activity
                .active_minutes
                .map(|minutes| (date, json!(minutes))),
            FitbitMetric::CardioMinutes => activity
                .cardio_minutes
                .map(|minutes| (date, json!(minutes))),
            FitbitMetric::PeakMinutes => {
                activity.peak_minutes.map(|minutes| (date, json!(minutes)))
            }
            _ => None,
        }
    }
}

type FitbitCache = HashMap<(FitbitMetric, NaiveDate), FitbitFetchedData>;

#[derive(Clone)]
enum FitbitFetchedData {
    Activity(FitbitActivityData),
    Sleep(FitbitSleepData),
}

#[derive(Clone, Copy)]
enum FitbitDataCategory {
    Sleep,
    Activity,
}

fn metric_category(metric: &FitbitMetric) -> FitbitDataCategory {
    match metric {
        FitbitMetric::SleepDuration
        | FitbitMetric::WakeUpTime
        | FitbitMetric::FellAsleepTime
        | FitbitMetric::SleepEfficiency
        | FitbitMetric::TotalDurationAwake => FitbitDataCategory::Sleep,
        FitbitMetric::Steps
        | FitbitMetric::CaloriesOut
        | FitbitMetric::ActiveMinutes
        | FitbitMetric::CardioMinutes
        | FitbitMetric::PeakMinutes => FitbitDataCategory::Activity,
    }
}
