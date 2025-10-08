use std::time::Duration as StdDuration;

use anyhow::anyhow;
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64_ENGINE;
use base64::Engine as _;
use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use lifestats_application::error::{ApplicationError, ApplicationResult};
use lifestats_application::integrations::fitbit::error::FitbitUpstreamError;
use lifestats_application::integrations::fitbit::ports::{
    FitbitActivityData, FitbitDataApi, FitbitOAuthApi, FitbitSleepData,
};
use lifestats_application::integrations::tokens::{AccessToken, FitbitTokenData, RefreshToken};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, RETRY_AFTER};
use reqwest::{Client, Method, StatusCode};
use serde::Deserialize;
use tracing::instrument;

const DEFAULT_API_BASE_URL: &str = "https://api.fitbit.com";
const TOKEN_PATH: &str = "/oauth2/token";

fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

pub struct HttpFitbitOAuthClient {
    http: Client,
    base_url: String,
    client_id: String,
    client_secret: Option<String>,
    redirect_uri: String,
}

impl HttpFitbitOAuthClient {
    pub fn new(
        http: Client,
        client_id: impl Into<String>,
        client_secret: Option<String>,
        redirect_uri: impl Into<String>,
    ) -> Self {
        Self {
            http,
            base_url: DEFAULT_API_BASE_URL.to_string(),
            client_id: client_id.into(),
            client_secret,
            redirect_uri: redirect_uri.into(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn auth_header(&self) -> Option<String> {
        self.client_secret.as_ref().map(|secret| {
            let credentials = format!("{}:{}", self.client_id, secret);
            format!("Basic {}", BASE64_ENGINE.encode(credentials.as_bytes()))
        })
    }

    async fn exchange(&self, form: Vec<(String, String)>) -> ApplicationResult<FitbitTokenData> {
        let url = format!("{}{}", self.base_url, TOKEN_PATH);
        let mut request = self
            .http
            .request(Method::POST, url)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&form);

        if let Some(header) = self.auth_header() {
            request = request.header(reqwest::header::AUTHORIZATION, header);
        }

        let response = request.send().await.map_err(|err| {
            ApplicationError::from(FitbitUpstreamError::UpstreamServiceError(err.into()))
        })?;

        let payload: OAuthTokenResponse = handle_response(response).await?;
        Ok(payload.into_token_data())
    }
}

#[async_trait]
impl FitbitOAuthApi for HttpFitbitOAuthClient {
    #[instrument(skip_all)]
    async fn exchange_code_for_token(
        &self,
        code: &str,
        pkce_verifier: Option<String>,
    ) -> ApplicationResult<FitbitTokenData> {
        let mut form = vec![
            ("grant_type".to_string(), "authorization_code".to_string()),
            ("code".to_string(), code.to_string()),
            ("client_id".to_string(), self.client_id.clone()),
            ("redirect_uri".to_string(), self.redirect_uri.clone()),
        ];

        if let Some(verifier) = pkce_verifier {
            form.push(("code_verifier".to_string(), verifier));
        }

        self.exchange(form).await
    }

    #[instrument(skip_all)]
    async fn refresh(&self, refresh_token: &RefreshToken) -> ApplicationResult<FitbitTokenData> {
        let form = vec![
            ("grant_type".to_string(), "refresh_token".to_string()),
            (
                "refresh_token".to_string(),
                refresh_token.as_str().to_string(),
            ),
            ("client_id".to_string(), self.client_id.clone()),
        ];

        self.exchange(form).await
    }
}

pub struct HttpFitbitDataClient {
    http: Client,
    base_url: String,
}

impl HttpFitbitDataClient {
    pub fn new(http: Client) -> Self {
        Self {
            http,
            base_url: DEFAULT_API_BASE_URL.to_string(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        access_token: &AccessToken,
        endpoint: String,
    ) -> ApplicationResult<T> {
        let response = self
            .http
            .get(endpoint)
            .bearer_auth(access_token.as_str())
            .send()
            .await
            .map_err(|err| {
                ApplicationError::from(FitbitUpstreamError::UpstreamServiceError(err.into()))
            })?;

        handle_response(response).await
    }
}

#[async_trait]
impl FitbitDataApi for HttpFitbitDataClient {
    #[instrument(skip_all)]
    async fn get_activity_data(
        &self,
        access_token: &AccessToken,
        date: NaiveDate,
    ) -> ApplicationResult<FitbitActivityData> {
        let url = format!(
            "{}/1/user/-/activities/date/{}.json",
            self.base_url,
            date.format("%Y-%m-%d")
        );

        let response: ActivitySummaryEnvelope = self.get_json(access_token, url).await?;
        Ok(response.into_activity_data())
    }

    #[instrument(skip_all)]
    async fn get_sleep_data(
        &self,
        access_token: &AccessToken,
        date: NaiveDate,
    ) -> ApplicationResult<FitbitSleepData> {
        let url = format!(
            "{}/1.2/user/-/sleep/date/{}.json",
            self.base_url,
            date.format("%Y-%m-%d")
        );

        let response: SleepEnvelope = self.get_json(access_token, url).await?;
        Ok(response.into_sleep_data())
    }
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    scope: String,
}

impl OAuthTokenResponse {
    fn into_token_data(self) -> FitbitTokenData {
        let expires_at = now_utc() + ChronoDuration::seconds(self.expires_in.max(0));

        let mut scopes: Vec<String> = self
            .scope
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        scopes.sort();
        scopes.dedup();

        FitbitTokenData {
            access_token: AccessToken::new(self.access_token),
            refresh_token: RefreshToken::new(self.refresh_token),
            expires_at,
            scopes,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ActivitySummaryEnvelope {
    summary: ActivitySummary,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivitySummary {
    #[serde(default)]
    steps: u32,
    #[serde(default)]
    calories_out: i32,
    #[serde(default)]
    lightly_active_minutes: i32,
    #[serde(default)]
    fairly_active_minutes: i32,
    #[serde(default)]
    very_active_minutes: i32,
    #[serde(default)]
    heart_rate_zones: Vec<HeartRateZone>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HeartRateZone {
    #[serde(default)]
    minutes: i32,
    name: HeartRateZoneName,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
enum HeartRateZoneName {
    #[serde(rename = "Out of Range")]
    OutOfRange,
    #[serde(rename = "Fat Burn")]
    FatBurn,
    Cardio,
    Peak,
}

impl ActivitySummaryEnvelope {
    fn into_activity_data(self) -> FitbitActivityData {
        let summary = self.summary;

        let active_minutes = summary
            .lightly_active_minutes
            .max(0)
            .saturating_add(summary.fairly_active_minutes.max(0))
            .saturating_add(summary.very_active_minutes.max(0)) as u32;

        let cardio_minutes = summary
            .heart_rate_zones
            .iter()
            .filter(|zone| zone.name == HeartRateZoneName::Cardio)
            .map(|zone| zone.minutes.max(0) as u32)
            .sum();

        let peak_minutes = summary
            .heart_rate_zones
            .iter()
            .filter(|zone| zone.name == HeartRateZoneName::Peak)
            .map(|zone| zone.minutes.max(0) as u32)
            .sum();

        FitbitActivityData {
            steps: Some(summary.steps),
            calories_out: Some(summary.calories_out.max(0) as u32),
            active_minutes: Some(active_minutes),
            cardio_minutes: Some(cardio_minutes),
            peak_minutes: Some(peak_minutes),
        }
    }
}

#[derive(Debug, Deserialize)]
struct SleepEnvelope {
    sleep: Vec<SleepEntry>,
    summary: SleepSummary,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SleepSummary {
    #[serde(default)]
    total_minutes_asleep: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SleepEntry {
    #[serde(default)]
    efficiency: u8,
    #[serde(default)]
    end_time: NaiveDateTime,
    #[serde(default)]
    is_main_sleep: bool,
    #[serde(default)]
    levels: SleepLevels,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SleepLevels {
    #[serde(default)]
    data: Vec<SleepLevelData>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SleepLevelData {
    date_time: NaiveDateTime,
    level: SleepStage,
    seconds: u32,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum SleepStage {
    Deep,
    Light,
    Rem,
    Wake,
}

impl SleepEnvelope {
    fn into_sleep_data(self) -> FitbitSleepData {
        let mut data = FitbitSleepData::default();
        if self.summary.total_minutes_asleep > 0 {
            data.total_duration_asleep = Some(ChronoDuration::minutes(
                self.summary.total_minutes_asleep as i64,
            ));
        }

        if let Some(main_sleep) = self.sleep.into_iter().find(|entry| entry.is_main_sleep) {
            data.sleep_efficiency = Some(main_sleep.efficiency);
            data.woke_up_at_utc = Some(Utc.from_utc_datetime(&main_sleep.end_time));

            if let Some(stage) = main_sleep
                .levels
                .data
                .iter()
                .find(|stage| stage.level != SleepStage::Wake && stage.seconds > 300)
            {
                data.fell_asleep_at_utc = Some(Utc.from_utc_datetime(&stage.date_time));
            }

            let awake_seconds: u32 = main_sleep
                .levels
                .data
                .iter()
                .filter(|stage| stage.level == SleepStage::Wake)
                .map(|stage| stage.seconds)
                .sum();

            if awake_seconds > 0 {
                data.total_duration_awake = Some(ChronoDuration::seconds(awake_seconds as i64));
            }
        }

        data
    }
}

async fn handle_response<T>(response: reqwest::Response) -> ApplicationResult<T>
where
    T: for<'de> Deserialize<'de>,
{
    let status = response.status();
    let headers = response.headers().clone();
    if status.is_success() {
        response.json::<T>().await.map_err(|err| {
            ApplicationError::from(FitbitUpstreamError::UpstreamServiceError(err.into()))
        })
    } else {
        let body = response.text().await.unwrap_or_else(|_| "".to_string());
        Err(ApplicationError::from(map_error(status, &headers, &body)))
    }
}

fn map_error(status: StatusCode, headers: &HeaderMap, body: &str) -> FitbitUpstreamError {
    if status == StatusCode::UNAUTHORIZED {
        FitbitUpstreamError::Unauthorized
    } else if status == StatusCode::TOO_MANY_REQUESTS {
        let retry_after = headers.get(RETRY_AFTER).and_then(parse_retry_after_header);
        FitbitUpstreamError::RateLimited { retry_after }
    } else if status == StatusCode::BAD_REQUEST && body_contains_invalid_grant(body) {
        FitbitUpstreamError::InvalidGrant
    } else {
        FitbitUpstreamError::UpstreamServiceError(anyhow!(
            "fitbit request failed: status={} body={}",
            status,
            body
        ))
    }
}

fn parse_retry_after_header(header: &HeaderValue) -> Option<StdDuration> {
    let value = header.to_str().ok()?;
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(StdDuration::from_secs(seconds));
    }

    DateTime::parse_from_rfc2822(value)
        .or_else(|_| DateTime::parse_from_rfc3339(value))
        .ok()
        .and_then(|dt| {
            let now = now_utc();
            let delta = dt.with_timezone(&Utc) - now;
            delta.to_std().ok()
        })
}

fn body_contains_invalid_grant(body: &str) -> bool {
    body.to_ascii_lowercase().contains("invalid_grant")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn activity_response_converts() {
        let payload = json!({
            "summary": {
                "steps": 5000,
                "caloriesOut": 2100,
                "lightlyActiveMinutes": 30,
                "fairlyActiveMinutes": 15,
                "veryActiveMinutes": 5,
                "heartRateZones": [
                    {"name": "Out of Range", "minutes": 400},
                    {"name": "Fat Burn", "minutes": 20},
                    {"name": "Cardio", "minutes": 10},
                    {"name": "Peak", "minutes": 5}
                ]
            }
        });

        let envelope: ActivitySummaryEnvelope = serde_json::from_value(payload).unwrap();
        let activity = envelope.into_activity_data();

        assert_eq!(activity.steps, Some(5000));
        assert_eq!(activity.calories_out, Some(2100));
        assert_eq!(activity.active_minutes, Some(50));
        assert_eq!(activity.cardio_minutes, Some(10));
        assert_eq!(activity.peak_minutes, Some(5));
    }

    #[test]
    fn sleep_response_converts() {
        let payload = json!({
            "summary": { "totalMinutesAsleep": 420 },
            "sleep": [
                {
                    "isMainSleep": true,
                    "efficiency": 95,
                    "duration": 25200000,
                    "minutesAwake": 30,
                    "startTime": "2024-10-01T22:30:00",
                    "endTime": "2024-10-02T06:30:00",
                    "levels": {
                        "data": [
                            {"dateTime": "2024-10-01T22:45:00", "level": "light", "seconds": 1200},
                            {"dateTime": "2024-10-01T23:05:00", "level": "deep", "seconds": 1800},
                            {"dateTime": "2024-10-02T06:00:00", "level": "wake", "seconds": 600}
                        ]
                    }
                }
            ]
        });

        let envelope: SleepEnvelope = serde_json::from_value(payload).unwrap();
        let sleep = envelope.into_sleep_data();

        assert_eq!(sleep.sleep_efficiency, Some(95));
        assert_eq!(sleep.total_duration_asleep.unwrap().num_minutes(), 420);
        assert!(sleep.fell_asleep_at_utc.is_some());
        assert!(sleep.woke_up_at_utc.is_some());
        assert_eq!(sleep.total_duration_awake.unwrap().num_minutes(), 10);
    }

    #[test]
    fn detects_invalid_grant_body() {
        assert!(body_contains_invalid_grant("{\"error\":\"invalid_grant\"}"));
        assert!(body_contains_invalid_grant("invalid_GRANT"));
        assert!(!body_contains_invalid_grant("something else"));
    }
}
