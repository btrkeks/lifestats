// use crate::plot::plot_types::Plot;
// use anyhow::{Result};
// use backend::util::paths;
// use serde::{Deserialize, Serialize};
// use std::fs;
// use backend::statistics::objects::StatisticId;
// use crate::plot::plot_types::AnyPlot;
// use cached::proc_macro::{cached};
//
// #[derive(Debug, Deserialize, Serialize)]
// struct Definitions {
//     version: String,
//     plots: Vec<AnyPlot>,
// }
//
// #[cached(result = true)]
// pub fn get_all_plots() -> Result<Vec<AnyPlot>> {
//     let definitions_path = paths::get_plot_definitions_path()?;
//     let json_content = fs::read_to_string(definitions_path)?;
//     let definitions: Definitions = serde_json::from_str(&json_content)?;
//     Ok(definitions.plots)
// }
//
// pub fn get_plots_for_id(stat_id: StatisticId) -> Result<Vec<AnyPlot>> {
//     // TODO: Improve efficiency?
//     Ok(get_all_plots()?
//         .into_iter()
//         .filter(|plot| plot.contains_statistic_with_id(stat_id))
//         .collect())
// }
