use anyhow::{anyhow, Context, Result};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::DurationSeconds;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use transfer_objects::statistic_id::StatisticId;

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiLoggerConfig {
    pub server_url: String,
    #[serde_as(as = "DurationSeconds<u64>")]
    pub heartbeat_timeout: Duration,
    pub statistic_id: StatisticId,
}

impl MidiLoggerConfig {
    pub fn new(
        server_url: impl Into<String>,
        heartbeat_timeout: Duration,
        statistic_id: StatisticId,
    ) -> Self {
        Self {
            server_url: server_url.into(),
            heartbeat_timeout,
            statistic_id,
        }
    }

    /// Load configuration from the default location
    pub fn load() -> Result<Self> {
        Self::load_from(Self::default_config_path()?)
    }

    /// Save configuration to the default location
    pub fn save(&self) -> Result<()> {
        self.save_to(Self::default_config_path()?)
    }

    /// Load configuration from a specific file
    pub fn load_from(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        debug!("Loading config from: {}", path.display());

        if !path.exists() {
            warn!("Config file not found at: {}", path.display());
            return Err(anyhow!("Config file not found at: {}", path.display()));
        }

        let content = fs::read_to_string(path).context("Failed to read config file")?;

        let config = serde_json::from_str(&content).context("Failed to parse config file")?;

        info!("Successfully loaded config from: {}", path.display());
        Ok(config)
    }

    /// Save configuration to a specific file
    pub fn save_to(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        debug!("Saving config to: {}", path.display());

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let content = serde_json::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(path, content).context("Failed to write config file")?;

        info!("Successfully saved config to: {}", path.display());
        Ok(())
    }

    fn default_config_path() -> Result<PathBuf> {
        let mut path =
            dirs::config_dir().ok_or_else(|| anyhow!("Could not determine config directory"))?;
        path.push("midi_logger");
        path.push("config.json");
        Ok(path)
    }
}
