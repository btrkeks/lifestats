use anyhow::{anyhow, Context, Result};
use dirs;
use std::fs;
use std::path::PathBuf;

pub fn get_project_data_dir() -> Result<PathBuf> {
    let mut path = dirs::data_local_dir().ok_or_else(|| anyhow!("Getting home directory"))?;

    path.push("lifestats");

    fs::create_dir_all(&path).context("Creating project data directory")?;

    Ok(path)
}

pub fn get_statistic_definitions_path() -> Result<PathBuf> {
    Ok(get_project_data_dir()?.join("definitions.json"))
}

pub fn get_plot_definitions_path() -> Result<PathBuf> {
    Ok(get_project_data_dir()?.join("plots.json"))
}

pub fn get_db_path() -> Result<PathBuf> {
    Ok(get_project_data_dir()?.join("lifestats.db"))
}

pub fn get_log_file() -> Result<PathBuf> {
    Ok(get_project_data_dir()?.join("lifestats.log"))
}

pub fn get_plot_path() -> Result<PathBuf> {
    Ok(get_project_data_dir()?.join("plots"))
}
