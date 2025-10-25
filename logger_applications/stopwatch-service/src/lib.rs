use anyhow::Result;
use zbus::{proxy, Connection};

/// Represents the current state of a stopwatch
#[derive(Debug, Clone)]
pub struct StopwatchStatus {
    pub statistic_name: String,
    pub is_running: bool,
    pub elapsed_seconds: i64,
}

impl StopwatchStatus {
    /// Format the elapsed time as HH:MM:SS
    pub fn format_duration(&self) -> String {
        let hours = self.elapsed_seconds / 3600;
        let minutes = (self.elapsed_seconds % 3600) / 60;
        let seconds = self.elapsed_seconds % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }

    /// Get a human-readable status string
    pub fn status_string(&self) -> String {
        if self.is_running {
            format!("{} {}", self.statistic_name, self.format_duration())
        } else {
            "No stopwatch running".to_string()
        }
    }
}

#[proxy(
    interface = "com.lifestats.Stopwatch1",
    default_service = "com.lifestats.Stopwatch",
    default_path = "/com/lifestats/Stopwatch"
)]
trait StopwatchProxy {
    /// Get the current state of the stopwatch
    fn get_state(&self) -> zbus::Result<(String, bool, i64)>;

    /// Signal emitted when stopwatch state changes
    #[zbus(signal)]
    fn state_changed(
        &self,
        statistic_name: &str,
        is_running: bool,
        elapsed_seconds: i64,
    ) -> zbus::Result<()>;
}

/// Client for interacting with the lifestats stopwatch DBus service
pub struct StopwatchClient {
    connection: Connection,
}

impl StopwatchClient {
    /// Create a new stopwatch client connected to the session bus
    pub async fn new() -> Result<Self> {
        let connection = Connection::session().await?;
        Ok(Self { connection })
    }

    /// Get the current stopwatch status
    pub async fn get_status(&self) -> Result<Option<StopwatchStatus>> {
        let proxy = StopwatchProxyProxy::new(&self.connection).await?;

        match proxy.get_state().await {
            Ok((statistic_name, is_running, elapsed_seconds)) => Ok(Some(StopwatchStatus {
                statistic_name,
                is_running,
                elapsed_seconds,
            })),
            Err(e) => {
                // Service not available
                log::debug!("Failed to get stopwatch state: {}", e);
                Ok(None)
            }
        }
    }

    /// Listen for stopwatch state changes and call the provided callback
    /// This function blocks until an error occurs or the stream ends
    pub async fn listen_for_updates<F>(&self, mut callback: F) -> Result<()>
    where
        F: FnMut(StopwatchStatus) + Send + 'static,
    {
        use futures_util::StreamExt;

        let proxy = StopwatchProxyProxy::new(&self.connection).await?;
        let mut stream = proxy.receive_state_changed().await?;

        while let Some(signal) = stream.next().await {
            let args = signal.args()?;
            let status = StopwatchStatus {
                statistic_name: args.statistic_name.to_string(),
                is_running: args.is_running,
                elapsed_seconds: args.elapsed_seconds,
            };
            callback(status);
        }

        Ok(())
    }
}
