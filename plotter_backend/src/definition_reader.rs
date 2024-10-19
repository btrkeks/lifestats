use crate::plots::AnyPlot;
use anyhow::{Context, Result};
use backend::paths;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Deserialize, Serialize)]
struct Definitions {
    version: String,
    plots: Vec<AnyPlot>,
}

pub fn get_all_plots() -> Result<Vec<AnyPlot>> {
    let definitions_path = paths::get_plot_definitions_path().context("Getting path")?;
    let json_content = fs::read_to_string(definitions_path).context("Reading plot definitions")?;
    let definitions: Definitions =
        serde_json::from_str(&json_content).context("Parsing plot definitions")?;
    Ok(definitions.plots)
}

pub fn get_plots_for_id(stat_id: u32) -> Result<Vec<AnyPlot>> {
    // TODO: implement
    unimplemented!()
}
