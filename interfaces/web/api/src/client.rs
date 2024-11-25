use crate::types::{LogRequest, LogResponse};
use anyhow::{Result};
use backend::statistics::objects::StatisticId;
use reqwest::{Client, StatusCode};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Invalid date: {0}")]
    DateError(String),
}

#[derive(Clone, Debug)]
pub struct ApiClient {
    base_url: String,
    client: Client,
}

impl ApiClient {
    /// Creates a new API client with custom settings
    pub fn new(base_url: impl Into<String>) -> Result<Self, ClientError> {
        Ok(Self {
            base_url: base_url.into(),
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(ClientError::RequestError)?,
        })
    }

    /// Creates a new API client for localhost
    pub fn new_local() -> Result<Self, ClientError> {
        Self::new("http://localhost:8000")
    }

    /// Sends a log entry to the API endpoint.
    ///
    /// # Arguments
    ///
    /// * `statistic_id` - The ID of the statistic to log
    /// * `value` - The value to log
    /// * `update_policy` - Optional update policy ("Replace", "Add", or "Average")
    /// * `date` - Optional date for the log entry (defaults to current date)
    ///
    /// # Returns
    ///
    /// Returns a Result containing the LogResponse on success, or a ClientError on failure.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use api_client::client::ApiClient;
    /// use backend::statistics::objects::StatisticId;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let client = ApiClient::new_local()?;
    ///
    ///     let result = client.log_statistic(
    ///         StatisticId(1),
    ///         "10.5",
    ///         Some("Replace"),
    ///         None
    ///     ).await?;
    ///
    ///     println!("Log result: {}", result.message);
    ///     Ok(())
    /// }
    /// ```
    pub async fn log_statistic(
        &self,
        statistic_id: StatisticId,
        value: &str,
        update_policy: Option<&str>,
        date: Option<time::Date>,
    ) -> Result<LogResponse, ClientError> {
        let endpoint = format!("{}/api/log", self.base_url.trim_end_matches('/'));

        let payload = LogRequest {
            statistic_id,
            value: value.to_string(),
            update_policy: update_policy.map(String::from),
            date: date.unwrap_or_else(|| time::OffsetDateTime::now_utc().date()),
        };

        let response = self.client
            .post(&endpoint)
            .json(&payload)
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                let response_data = response.json::<LogResponse>().await?;
                if !response_data.success {
                    return Err(ClientError::ServerError(response_data.message));
                }
                Ok(response_data)
            },
            status if status.is_client_error() => {
                let error_response = response.json::<LogResponse>().await
                    .unwrap_or_else(|_| LogResponse {
                        success: false,
                        message: "Bad Request".to_string(),
                    });
                Err(ClientError::ServerError(error_response.message))
            },
            status if status.is_server_error() => {
                Err(ClientError::ServerError("Server error occurred".to_string()))
            },
            _ => Err(ClientError::ServerError("Unexpected response".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};
    use serde_json::json;

    #[tokio::test]
    async fn test_successful_log() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/log"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "message": "Successfully logged value"
            })))
            .mount(&mock_server)
            .await;

        let client = ApiClient::new(mock_server.uri()).unwrap();

        let result = client.log_statistic(
            StatisticId(1),
            "10.5",
            Some("Replace"),
            None,
        ).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.success);
        assert_eq!(response.message, "Successfully logged value");
    }

    #[tokio::test]
    async fn test_server_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/log"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let client = ApiClient::new(mock_server.uri()).unwrap();

        let result = client.log_statistic(
            StatisticId(1),
            "10.5",
            None,
            None,
        ).await;

        assert!(matches!(result, Err(ClientError::ServerError(_))));
    }

    #[tokio::test]
    async fn test_bad_request() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/log"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "success": false,
                "message": "Invalid input"
            })))
            .mount(&mock_server)
            .await;

        let client = ApiClient::new(mock_server.uri()).unwrap();

        let result = client.log_statistic(
            StatisticId(999),
            "invalid",
            None,
            None,
        ).await;

        assert!(matches!(result, Err(ClientError::ServerError(_))));
    }
}