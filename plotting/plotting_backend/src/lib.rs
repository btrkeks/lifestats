// plotter/src/lib.rs
use crate::plots::plot_types::Plot;
use std::path::Path;
use anyhow::Result;
use chrono::Local;

pub mod plots;
pub mod plotters_backend;
mod util;

use backend::util::paths::{get_db_path, get_plot_path};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rayon::prelude::*;
use rusqlite::OpenFlags;
use backend::statistics::objects::StatisticId;
use crate::plots::plot_getter::{get_all_plots, get_plots_for_id};
use crate::plots::plot_types::AnyPlot;
use memory_stats::memory_stats;
use backend::database::db_ops::open_db_readonly;

fn log_memory_usage(context: &str) {
    if let Some(usage) = memory_stats() {
        eprintln!(
            "Memory usage at {}: Physical: {} bytes, Virtual: {} bytes",
            context,
            usage.physical_mem,
            usage.virtual_mem
        );
    }
}

fn regenerate_plots(save_dir: &Path, plots: Vec<AnyPlot>) -> Result<()> {
    let end_date = Local::now().date_naive();
    let start_date = end_date - chrono::Duration::days(14); // TODO: 14 should be configurable

    plots.par_iter().try_for_each(|plot| -> Result<()> {
        let conn = open_db_readonly()?;
        plot.plot(&save_dir, &conn, start_date, end_date)
    })
}

pub fn regenerate_plots_for_id(stat_id: StatisticId) -> Result<()> {
    let plot_dir = get_plot_path()?;
    let plots = get_plots_for_id(stat_id)?;
    regenerate_plots(&plot_dir, plots)
}

pub fn regenerate_all_plots() -> Result<()> {
    let plot_dir = get_plot_path()?;
    regenerate_plots(&plot_dir, get_all_plots()?)
}
