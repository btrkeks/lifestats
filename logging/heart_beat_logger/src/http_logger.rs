use std::io::Write;
use anyhow::{anyhow, Context, Result};
use api_client::client::{ApiClient, ClientError};
use backend::statistics::objects::{StatisticId, TimeFrame};
use backend::statistics::update_policy::TimeFrameListUpdatePolicy;
use std::sync::Arc;
use tokio::runtime::Runtime;

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;

/// A TimeFrameLogger implementation that sends logs to a remote server using HTTP.
/// Uses exponential backoff for retries on failure.
#[derive(Clone)]
pub struct HttpLogger {
    client: Arc<ApiClient>,
    runtime: Arc<Runtime>,
}

impl HttpLogger {
    /// Creates a new HttpLogger with a custom server URL
    pub fn new(server_url: &str) -> Result<Self> {
        Ok(Self {
            client: Arc::new(ApiClient::new(server_url).context("Failed to create API client")?),
            runtime: Arc::new(Runtime::new().context("Failed to create Tokio runtime")?),
        })
    }

    /// Creates a new HttpLogger for localhost (http://localhost:8000)
    pub fn new_local() -> Result<Self> {
        Self::new("http://localhost:8000")
    }

    fn log_with_retry(
        &self,
        time_frame: TimeFrame,
        statistic_id: StatisticId,
    ) -> Result<()> {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            let result = self.runtime.block_on(async {
                self.client
                    .log_statistic(
                        statistic_id,
                        &time_frame.to_string(),
                        Some(TimeFrameListUpdatePolicy::OverwriteLast.to_string().as_str()),
                        None,
                    )
                    .await
            });

            match result {
                Ok(_) => return Ok(()),
                Err(ClientError::RequestError(e)) if e.is_timeout() || e.is_connect() => {
                    // Network-related errors are retryable
                    last_error = Some(e);
                    if attempt < MAX_RETRIES - 1 {
                        std::thread::sleep(std::time::Duration::from_millis(
                            RETRY_DELAY_MS * 2_u64.pow(attempt),
                        ));
                    }
                }
                Err(e) => {
                    // Other errors (like validation errors) should fail immediately
                    return Err(anyhow!("Failed to log time frame: {}", e));
                }
            }
        }

        Err(anyhow!(
            "Failed to log time frame after {} retries. Last error: {}",
            MAX_RETRIES,
            last_error.unwrap()
        ))
    }
}

impl super::TimeFrameLogger for HttpLogger {
    fn log(&self, time_frame: TimeFrame, statistic_id: StatisticId) -> Result<()> {
        log::debug!(
            "Logging time frame {:?} for statistic {}",
            time_frame,
            statistic_id
        );

        match self.log_with_retry(time_frame, statistic_id) {
            Ok(_) => {
                print!("O");
                std::io::stdout().flush().unwrap_or_default();

                log::debug!("Successfully logged time frame");
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to log time frame: {}", e);
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};
    use serde_json::json;
    use crate::TimeFrameLogger;

    #[tokio::test]
    async fn test_successful_log() {
        // Start mock server
        let mock_server = MockServer::start().await;

        // Create success response mock
        Mock::given(method("POST"))
            .and(path("/api/log"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "message": "Successfully logged value"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Create logger with mock server URL
        let logger = HttpLogger::new(&mock_server.uri()).unwrap();

        // Create test time frame
        let time_frame = TimeFrame::new(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
        ).unwrap();

        // Attempt to log
        let result = logger.log(time_frame, StatisticId(1));
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_retry_behavior() {
        let mock_server = MockServer::start().await;

        // First two calls fail, third succeeds
        Mock::given(method("POST"))
            .and(path("/api/log"))
            .respond_with(ResponseTemplate::new(500))
            .expect(2)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/api/log"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "success": true,
                "message": "Successfully logged value"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let logger = HttpLogger::new(&mock_server.uri()).unwrap();
        let time_frame = TimeFrame::new(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
        ).unwrap();

        let result = logger.log(time_frame, StatisticId(1));
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_max_retries_exceeded() {
        let mock_server = MockServer::start().await;

        // All calls fail
        Mock::given(method("POST"))
            .and(path("/api/log"))
            .respond_with(ResponseTemplate::new(500))
            .expect(MAX_RETRIES as usize)
            .mount(&mock_server)
            .await;

        let logger = HttpLogger::new(&mock_server.uri()).unwrap();
        let time_frame = TimeFrame::new(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
        ).unwrap();

        let result = logger.log(time_frame, StatisticId(1));
        assert!(result.is_err());
    }
}