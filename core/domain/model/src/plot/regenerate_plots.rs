// plotter/src/lib.rs
use std::path::Path;
use anyhow::Result;
use chrono::Local;

pub mod plot;
pub mod plotters_backend;
mod util;
mod repositories;
mod objects;
mod type_definitions;
mod backends;
mod plottable;

use rayon::prelude::*;
use crate::repositories::image_repository::ImageRepository;
use crate::repositories::plots_repository::PlotRepository;

fn regenerate_plots(plots: Vec<plot::AnyPlot>,
                    image_repository: &Box<dyn ImageRepository>) -> Result<()> {
    let end_date = Local::now().date_naive();
    let start_date = end_date - chrono::Duration::days(14); // TODO: 14 should be configurable

    plots.par_iter().try_for_each(|plot| -> Result<()> {
        // let image = plot.plot(start_date, end_date);
        // image_repository.save(image)?;
        unimplemented!()
    })
}

pub fn regenerate_plots_for_id(stat_id: u64,
                               plot_repository: &Box<dyn PlotRepository>,
                               image_repository: &Box<dyn ImageRepository>) -> Result<()> {
    let plots = plot_repository.get_for_statistic_id(stat_id)?;
    regenerate_plots(plots, image_repository)
}

pub fn regenerate_all_plots(plot_repository: &Box<dyn PlotRepository>,
                            image_repository: &Box<dyn ImageRepository>) -> Result<()> {
    let plots = plot_repository.get_all()?;
    regenerate_plots(plots, image_repository)
}
