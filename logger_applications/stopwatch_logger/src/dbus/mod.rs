use anyhow::Result;
use lifestats_application::HeartbeatUpdate;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use zbus::object_server::SignalEmitter;
use zbus::{interface, Connection};

#[derive(Debug, Clone)]
pub struct StopwatchState {
    pub statistic_name: String,
    pub is_running: bool,
    pub elapsed_seconds: i64,
}

#[derive(Clone)]
struct StopwatchDBusService {
    state: Arc<RwLock<StopwatchState>>,
}

#[interface(name = "com.lifestats.Stopwatch1")]
impl StopwatchDBusService {
    /// Get current stopwatch state
    async fn get_state(&self) -> (String, bool, i64) {
        let state = self.state.read().await;
        (
            state.statistic_name.clone(),
            state.is_running,
            state.elapsed_seconds,
        )
    }

    /// Signal emitted when state changes
    #[zbus(signal)]
    async fn state_changed(
        emitter: &SignalEmitter<'_>,
        statistic_name: &str,
        is_running: bool,
        elapsed_seconds: i64,
    ) -> zbus::Result<()>;
}

pub struct DBusStatePublisher {
    statistic_name: String,
    service: StopwatchDBusService,
    connection: Connection,
}

impl DBusStatePublisher {
    pub async fn new(statistic_name: String) -> Result<Self> {
        let connection = Connection::session().await?;

        let state = Arc::new(RwLock::new(StopwatchState {
            statistic_name: statistic_name.clone(),
            is_running: false,
            elapsed_seconds: 0,
        }));

        let service = StopwatchDBusService {
            state: state.clone(),
        };

        connection
            .object_server()
            .at("/com/lifestats/Stopwatch", service.clone())
            .await?;

        connection.request_name("com.lifestats.Stopwatch").await?;

        log::info!("DBus service registered at com.lifestats.Stopwatch");

        Ok(Self {
            statistic_name,
            service,
            connection,
        })
    }

    pub async fn run(self, mut updates_rx: broadcast::Receiver<HeartbeatUpdate>) -> Result<()> {
        log::info!("DBus publisher started, waiting for updates...");

        loop {
            match updates_rx.recv().await {
                Ok(update) => {
                    log::debug!("Received update: {:?}", update);

                    let (is_running, elapsed_seconds) = {
                        let state = self.service.state.read().await;
                        match update {
                            HeartbeatUpdate::DurationChanged(duration) => {
                                (true, duration.num_seconds())
                            }
                            HeartbeatUpdate::Started => (true, state.elapsed_seconds),
                            HeartbeatUpdate::Stopped => (false, state.elapsed_seconds),
                        }
                    };

                    // Update internal state for GetState queries
                    {
                        let mut state = self.service.state.write().await;
                        state.is_running = is_running;
                        state.elapsed_seconds = elapsed_seconds;
                    }

                    // Emit DBus signal using the values we just calculated
                    let object_server = self.connection.object_server();
                    let iface_ref = object_server
                        .interface::<_, StopwatchDBusService>("/com/lifestats/Stopwatch")
                        .await?;

                    StopwatchDBusService::state_changed(
                        &iface_ref.signal_emitter(),
                        &self.statistic_name,
                        is_running,
                        elapsed_seconds,
                    )
                    .await?;

                    log::debug!(
                        "Emitted signal: {} {} {}s",
                        self.statistic_name,
                        if is_running { "running" } else { "stopped" },
                        elapsed_seconds
                    );
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    log::warn!("DBus publisher lagged, skipped {} messages", skipped);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    log::info!("DBus publisher channel closed, shutting down");
                    break;
                }
            }
        }

        log::info!("DBus publisher shut down");
        Ok(())
    }
}
