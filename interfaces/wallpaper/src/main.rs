/// Description:
/// This application displays plots for statistics as a wallpaper.
/// It listenes to new data and automatically regenerates the plots accordingly.
mod create_wallpaper;
mod mutable_image;
mod plot_drawing;
mod plot_position;
mod plot_position_repository;
mod set_wallpaper;

use crate::create_wallpaper::PlotPositionRepositoryImpl;
use crate::mutable_image::MutableImage;
use crate::plot_drawing::{initiate_wallpapers, redraw_all_plots, redraw_plot};
use crate::set_wallpaper::set_wallpaper;
use anyhow::Result;
use events::event_port::{Event, EventHandler, EventSubscriber};
use events::unix_socket::UnixSocketEvent;
use lifestats_core::app_context::{build_application, AppContext};
use log::{debug, error, info};
use util::stdout_logging::init_logging;

struct RedrawEventHandler {
    app_context: AppContext,
    wallpapers: Box<[MutableImage]>,
    plot_position_repository: PlotPositionRepositoryImpl,
}

impl RedrawEventHandler {
    pub fn new(
        app_context: AppContext,
        plot_position_repository: PlotPositionRepositoryImpl,
    ) -> Result<Self> {
        // TODO: Maybe this doesn't belong here
        let wallpapers =
            initiate_wallpapers(&app_context, &plot_position_repository).map_err(|e| {
                error!("Failed to initiate wallpapers: {}", e);
                e
            })?;

        set_desktop_wallpapers(&wallpapers).map_err(|e| {
            error!("Failed to set wallpapers: {}", e);
            e
        })?;

        Ok(Self {
            app_context,
            wallpapers,
            plot_position_repository,
        })
    }
}

pub fn set_desktop_wallpapers(wallpapers: &[MutableImage]) -> Result<()> {
    // TODO: Make this configurable
    set_wallpaper(&wallpapers[0], "HDMI-2")?;
    set_wallpaper(&wallpapers[1], "HDMI-1")?;
    Ok(())
}

impl EventHandler for RedrawEventHandler {
    fn handle_event(&mut self, event: Event) {
        debug!("Received event: {:?}", event);

        match event {
            Event::ValueUpdated(id) | Event::GoalUpdated(id) => {
                if let Err(e) = redraw_plot(
                    &self.app_context,
                    &mut self.wallpapers,
                    id,
                    &self.plot_position_repository,
                ) {
                    error!("Failed to update wallpaper for id {}: '{}'", id, e);
                }

                set_desktop_wallpapers(&self.wallpapers).unwrap();
            }
            Event::NewDay => {
                // Need to update all statistics
                if let Err(e) = redraw_all_plots(
                    &self.app_context,
                    &mut self.wallpapers,
                    &self.plot_position_repository,
                ) {
                    error!("Failed to update all plots: '{}'", e);
                }

                set_desktop_wallpapers(&self.wallpapers).unwrap();
            }
        }
    }
}

fn main() -> Result<()> {
    init_logging();
    info!("Starting wallpaper updater");

    let socket = UnixSocketEvent::default();
    let app_context = build_application()?;
    let plot_position_repoitory = PlotPositionRepositoryImpl::new();
    let event_handler = RedrawEventHandler::new(app_context, plot_position_repoitory)?;

    socket.listen_for_events(event_handler)?.join().unwrap();

    info!("Closing wallpaper updater");
    Ok(())
}
