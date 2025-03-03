use crate::logger::logger_trait::TimeFrameLogger;
use anyhow::{anyhow, Context, Result};
use chrono::NaiveTime;
use lifestats_core::TimeFrameListUpdatePolicy;
use std::io::Write;
use std::sync::Arc;
use tokio::runtime::Runtime;
use transfer_objects::statistic_id::StatisticId;
use web_api::client::{ApiClient, ClientError};

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;

/// A TimeFrameLogger implementation that sends logs to a remote server using HTTP.
/// Uses exponential backoff for retries on failure.
#[derive(Clone)]
pub struct HttpLogger {
    statistic_id: StatisticId,
    client: Arc<ApiClient>,
    runtime: Arc<Runtime>,
}

impl HttpLogger {
    /// Creates a new HttpLogger with a custom server URL
    pub fn new(statistic_id: StatisticId, server_url: &str) -> anyhow::Result<Self> {
        Ok(Self {
            statistic_id,
            client: Arc::new(ApiClient::new(server_url).context("Failed to create API client")?),
            runtime: Arc::new(Runtime::new().context("Failed to create Tokio runtime")?),
        })
    }

    fn log_with_retry(&self, start_time: NaiveTime, end_time: NaiveTime) -> Result<()> {
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            let result = self.runtime.block_on(async {
                self.client
                    .log_time_frame(
                        self.statistic_id,
                        start_time,
                        end_time,
                        Some(TimeFrameListUpdatePolicy::UpdateEndValue),
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

impl TimeFrameLogger for HttpLogger {
    fn log(&self, start_time: NaiveTime, end_time: NaiveTime) -> Result<()> {
        match self.log_with_retry(start_time, end_time) {
            Ok(_) => {
                print!("O");
                let _ = std::io::stdout().flush();

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
