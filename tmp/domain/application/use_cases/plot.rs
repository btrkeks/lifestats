use crate::app_context::AppContext;
use anyhow::Result;
use chrono::Local;
use domain_models::plot::any_plot::AnyPlot;
use log::debug;
use transfer_objects::plot_image::PlotImage;
use transfer_objects::statistic_id::StatisticId;

fn plot_plots(plots: Box<[AnyPlot]>) -> Result<Box<[PlotImage]>> {
    // TODO: This should be configurable
    let end_date = Local::now().date_naive();
    let start_date = end_date - chrono::Duration::days(14);

    // TODO: This could be parallelized
    plots
        .into_iter()
        .map(|plot| {
            debug!("Plotting plot: {:?}", plot.get_title());
            Ok(PlotImage {
                plot_id: plot.get_id(),
                image: plot.plot(start_date, end_date)?,
            })
        })
        .collect()
}

pub fn plot_all_containing_id(
    app_context: &AppContext,
    stat_id: StatisticId,
) -> Result<Box<[PlotImage]>> {
    let plots = app_context.plot_repository.get_for_statistic_id(stat_id)?;
    plot_plots(plots)
}

pub fn plot_all(app_context: &AppContext) -> Result<Box<[PlotImage]>> {
    let plots = app_context.plot_repository.get_all()?;
    plot_plots(plots)
}
