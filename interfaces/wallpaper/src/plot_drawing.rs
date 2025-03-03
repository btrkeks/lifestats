use std::iter;
use crate::create_wallpaper::{get_background_image, update_wallpaper_with_images};
use crate::mutable_image::MutableImage;
use crate::plot_position_repository::PlotPositionRepository;
use lifestats_core::app_context::AppContext;
use lifestats_core::plot::{plot_all, plot_all_containing_id};
use transfer_objects::statistic_id::StatisticId;
use anyhow::Result;
use anyhow::Context;

// TODO: Might want to turn wallpapers into an own struct
pub fn initiate_wallpapers(
    app_context: &AppContext,
    plot_position_repository: &impl PlotPositionRepository,
) -> Result<Box<[MutableImage]>> {
    let mut wallpapers = create_wallpapers(2) // TODO: Num should be configurable
        .context("Failed to create wallpapers")?;
    redraw_all_plots(app_context, &mut wallpapers, plot_position_repository)
        .context("Failed to redraw all plots")?;

    Ok(wallpapers)
}

pub fn create_wallpapers(num: u32) -> Result<Box<[MutableImage]>> {
    let background_image: MutableImage = get_background_image()?;

    Ok(iter::repeat(background_image)
        .take(num as usize)
        .collect())
}

pub fn redraw_all_plots(
    app_context: &AppContext,
    wallpapers: &mut [MutableImage],
    plot_position_repository: &impl PlotPositionRepository,
) -> Result<()> {
    let plot_images = plot_all(app_context)
        .context("Failed to plot all statistics")?;
    update_wallpaper_with_images(wallpapers, plot_images, plot_position_repository)
        .context("Failed to update wallpapers with plot images")
}

pub fn redraw_plot(
    app_context: &AppContext,
    wallpapers: &mut [MutableImage],
    stat_id: StatisticId,
    plot_position_repository: &impl PlotPositionRepository,
) -> Result<()> {
    let plot_images = plot_all_containing_id(app_context, stat_id)
        .with_context(|| format!("Failed to plot statistic with id {}", stat_id))?;
    update_wallpaper_with_images(wallpapers, plot_images, plot_position_repository)
        .context("Failed to update wallpapers with plot images")
}
