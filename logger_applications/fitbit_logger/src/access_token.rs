use ini::Ini;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AccessTokenError {
    #[error("Home directory not found")]
    HomeDirectoryNotFound,

    #[error("Failed to load config file: {0}")]
    ConfigLoadError(#[from] ini::Error),

    #[error("ACCESS_TOKEN not found in config.ini")]
    AccessTokenNotFound,
}

fn get_config_path() -> Result<PathBuf, AccessTokenError> {
    dirs::home_dir()
        .ok_or(AccessTokenError::HomeDirectoryNotFound)
        .map(|home| home.join(".config").join("lifestats").join("config.ini"))
}

pub fn get_access_token() -> Result<String, AccessTokenError> {
    // TODO: Rework this
    let config_path = get_config_path()?;
    let config = Ini::load_from_file(config_path)?;

    config
        .get_from(Some("Fitbit"), "ACCESS_TOKEN")
        .ok_or(AccessTokenError::AccessTokenNotFound)
        .map(String::from)
}
