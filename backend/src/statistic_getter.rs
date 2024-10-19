use crate::paths;
use crate::statistics::Statistic;
use crate::statistics::{AnyStatistic, TimeAmountStatistic, TimeSpansStatistic};
use anyhow::{anyhow, Context, Result};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::{RwLock, RwLockReadGuard};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StatisticGetterError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    JsonParseError(#[from] serde_json::Error),
    #[error("Statistic not found: {0}")]
    StatisticNotFound(u32),
    #[error("Statistics not found: {0:?}")]
    StatisticsNotFound(Vec<u32>),
}

#[derive(Debug, Deserialize, Serialize)]
struct Definitions {
    version: String,
    last_updated: String,
    statistics: Vec<AnyStatistic>,
}

lazy_static! {
    static ref STATISTICS_CACHE: RwLock<Option<Vec<AnyStatistic>>> = RwLock::new(None);
}

fn load_statistics_from_file() -> Result<Vec<AnyStatistic>> {
    let definitions_path = paths::get_statistic_definitions_path().context("Getting path")?;
    let json_content =
        fs::read_to_string(definitions_path).context("Reading statistic definitions")?;
    let definitions: Definitions =
        serde_json::from_str(&json_content).context("Parsing statistic definitions")?;
    Ok(definitions.statistics)
}

// TODO: refactor
fn read_all_statistics_cache() -> Result<RwLockReadGuard<'static, Option<Vec<AnyStatistic>>>> {
    {
        let cache = STATISTICS_CACHE
            .read()
            .map_err(|e| anyhow!("Failed to acquire read lock: {}", e))?;
        if cache.is_some() {
            return Ok(cache);
        }
    } // Drop the read lock

    {
        let mut cache = STATISTICS_CACHE
            .write()
            .map_err(|e| anyhow!("Failed to acquire write lock: {}", e))?;
        if cache.is_none() {
            *cache = Some(
                load_statistics_from_file().context("Reading statistic definitions from json")?,
            );
        }
    }

    let cache = STATISTICS_CACHE
        .read()
        .map_err(|e| anyhow!("Failed to acquire read lock: {}", e))?;
    Ok(cache)
}

pub fn get_all_statistics() -> Result<Vec<AnyStatistic>> {
    let cache = read_all_statistics_cache()?;
    let statistics = cache.as_ref().unwrap();
    Ok(statistics.clone())
}

pub fn get_statistic_by_id(id: u32) -> Result<Option<AnyStatistic>> {
    let cache = read_all_statistics_cache()?;
    let statistics = cache.as_ref().unwrap();
    Ok(statistics
        .iter()
        .find(|stat| stat.get_id() == id)
        .map(|stat| stat.clone()))
}

pub fn get_statistics_by_ids(ids: &[u32]) -> Result<Vec<AnyStatistic>> {
    let cache = read_all_statistics_cache()?;
    let statistics = cache.as_ref().unwrap();

    let mut found_statistics = Vec::new();
    let mut not_found_ids = Vec::new();

    for &id in ids {
        match statistics.iter().find(|stat| stat.get_id() == id) {
            Some(stat) => found_statistics.push(stat.clone()),
            None => not_found_ids.push(id),
        }
    }

    if !not_found_ids.is_empty() {
        return Err(anyhow!("Statistics not found"));
    }

    Ok(found_statistics)
}

pub fn get_all_time_amount_statistics() -> Result<Vec<TimeAmountStatistic>> {
    let cache = read_all_statistics_cache()?;
    let statistics = cache.as_ref().unwrap();

    let amount_statistics: Vec<TimeAmountStatistic> = statistics
        .iter()
        .filter_map(|stat| {
            if let AnyStatistic::TimeAmount(a) = stat {
                Some(a.clone())
            } else {
                None
            }
        })
        .collect();

    Ok(amount_statistics)
}

pub fn get_all_time_spans_statistics() -> Result<Vec<TimeSpansStatistic>> {
    let cache = read_all_statistics_cache()?;
    let statistics = cache.as_ref().unwrap();

    let amount_statistics: Vec<TimeSpansStatistic> = statistics
        .iter()
        .filter_map(|stat| {
            if let AnyStatistic::TimeSpans(a) = stat {
                Some(a.clone())
            } else {
                None
            }
        })
        .collect();

    Ok(amount_statistics)
}

#[test]
fn test_parse_example_statistics_without_error() {
    get_all_statistics().unwrap();
}
