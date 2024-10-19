use std::os::linux::raw::stat;
// plotter/src/main.rs
use crate::definition_reader::{get_all_plots, get_plots_for_id};
use crate::plots::{AnyPlot, Plot};
use anyhow::Result;
use chrono::Local;

pub mod color_consts;
mod common;
mod definition_reader;
mod plot_data;
mod plots;
mod plotters_backend;

use backend::paths::get_db_path;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rayon::prelude::*;
use rusqlite::OpenFlags;

fn regenerate_plots(plots: Vec<AnyPlot>) -> Result<()> {
    let end_date = Local::now().date_naive();
    let start_date = end_date - chrono::Duration::days(14); // TODO: 14 should be configurable

    let manager =
        SqliteConnectionManager::file(get_db_path()?).with_flags(OpenFlags::SQLITE_OPEN_READ_ONLY);
    let pool = Pool::new(manager)?;

    plots.par_iter().try_for_each(|plot| -> Result<()> {
        let conn = pool.get()?;
        plot.plot(&conn, start_date, end_date)
    })
}

pub fn regenerate_plots_for_id(stat_id: u32) -> Result<()> {
    regenerate_plots(get_plots_for_id(stat_id)?)
}

pub fn regenerate_all_plots() -> Result<()> {
    regenerate_plots(get_all_plots()?)
}
