use heart_beat_logger::logger::local_logger::LocalLogger;
use heart_beat_logger::{HeartbeatConfig, HeartbeatLogger};
use lifestats_core::app_context::AppContext;
use rocket::yansi::Paint;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use testing_utilities::time_provider::RealTimeProvider;
use tokio::sync::RwLock;
use transfer_objects::statistic_id::StatisticId;

pub struct HeartbeatManager {
    loggers:
        RwLock<HashMap<StatisticId, Arc<Mutex<HeartbeatLogger<LocalLogger, RealTimeProvider>>>>>,
    app_context: Arc<AppContext>,
}

impl HeartbeatManager {
    pub fn new(app_context: Arc<AppContext>) -> Self {
        Self {
            loggers: RwLock::new(HashMap::new()),
            app_context,
        }
    }

    pub async fn get_or_create_logger(
        &self,
        stat_id: StatisticId,
    ) -> anyhow::Result<Arc<Mutex<HeartbeatLogger<LocalLogger, RealTimeProvider>>>> {
        // Try to get existing logger
        if let Some(logger) = self.loggers.read().await.get(&stat_id) {
            return Ok(logger.clone());
        }

        // Create new logger
        let mut loggers = self.loggers.write().await;
        let logger = Arc::new(Mutex::new(
            HeartbeatLogger::<LocalLogger, RealTimeProvider>::new_with_local_logger(
                Arc::new(RealTimeProvider),
                Arc::clone(&self.app_context),
                stat_id,
                HeartbeatConfig {
                    // TODO: This should be configurable
                    idle_timeout: chrono::Duration::seconds(20),
                    flush_interval: chrono::Duration::seconds(10),
                },
            ),
        ));

        loggers.insert(stat_id, logger.clone());
        Ok(logger)
    }
}
