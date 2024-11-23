use crate::statistics::type_definitions::{AnyStatistic, Statistic};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use thiserror::Error;
use crate::util::paths;
use cached::proc_macro::{cached};
use crate::statistics::objects::StatisticId;
use crate::statistics::type_definitions::time_amount::TimeAmountStatistic;
use crate::statistics::type_definitions::time_spans::TimeSpansStatistic;

#[derive(Error, Debug, Clone)]
pub enum StatisticGetterError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("JSON parse error: {0}")]
    JsonParseError(String),
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

#[cached(result = true)]
pub fn get_all_statistics() -> Result<Vec<AnyStatistic>, StatisticGetterError> {
    let definitions_path = paths::get_statistic_definitions_path()
        .map_err(|e| StatisticGetterError::IoError(e.to_string()))?;

    let json_content = fs::read_to_string(definitions_path)
        .map_err(|e| StatisticGetterError::IoError(e.to_string()))?;

    let definitions: Definitions = serde_json::from_str(&json_content)
        .map_err(|e| StatisticGetterError::JsonParseError(e.to_string()))?;

    Ok(definitions.statistics)
}

pub fn get_statistic_by_id(id: StatisticId) -> Result<Option<AnyStatistic>> {
    get_all_statistics()?.into_iter().find(|stat| stat.get_id() == id).map(Ok).transpose()
}

pub fn get_statistics_by_ids(ids: &[u32]) -> Result<Vec<AnyStatistic>> {
    let statistics = get_all_statistics()?;
    let found: Vec<_> = statistics
        .into_iter()
        .filter(|stat| ids.contains(&stat.get_id().0))
        .collect();

    if found.len() != ids.len() {
        return Err(anyhow!("Statistics not found"));
    }
    Ok(found)
}

pub fn get_all_time_amount_statistics() -> Result<Vec<TimeAmountStatistic>> {
    Ok(get_all_statistics()?
        .into_iter()
        .filter_map(|stat| match stat {
            AnyStatistic::TimeAmount(a) => Some(a),
            _ => None
        })
        .collect())
}

pub fn get_all_time_spans_statistics() -> Result<Vec<TimeSpansStatistic>> {
    Ok(get_all_statistics()?
        .into_iter()
        .filter_map(|stat| match stat {
            AnyStatistic::TimeSpans(a) => Some(a),
            _ => None
        })
        .collect())
}

pub fn store_statistic(_stat: AnyStatistic) -> Result<()> {
    // TODO: Implement
    unimplemented!();
}

#[test]
fn test_parse_example_statistics_without_error() {
    get_all_statistics().unwrap();
}
