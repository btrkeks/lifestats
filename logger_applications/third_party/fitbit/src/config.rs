use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use transfer_objects::statistic_id::StatisticId;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum FitbitMetric {
    // Sleep metrics
    SleepDuration,
    WakeUpTime,
    FellAsleepTime,
    SleepEfficiency,
    TotalDurationAwake,

    // Activity metrics
    Steps,
    CaloriesOut,
    ActiveMinutes,

    // Heart rate zones
    CardioMinutes,
    PeakMinutes,
}

#[derive(Debug, Default)]
pub struct FitbitLoggingConfig {
    pub metric_map: HashMap<FitbitMetric, StatisticId>,
}

impl FitbitLoggingConfig {
    /// Creates a new empty configuration
    pub fn new() -> Self {
        Self {
            metric_map: HashMap::with_capacity(3), // Pre-allocate for typical use
        }
    }

    /// Adds or updates a metric mapping
    pub fn add_mapping(&mut self, metric: FitbitMetric, statistic_id: StatisticId) {
        self.metric_map.insert(metric, statistic_id);
    }

    /// Removes a metric mapping
    pub fn remove_mapping(&mut self, metric: &FitbitMetric) {
        self.metric_map.remove(metric);
    }

    /// Gets the configuration for a metric if it's active
    pub fn get_config(&self, metric: &FitbitMetric) -> Option<&StatisticId> {
        self.metric_map.get(metric)
    }
}
