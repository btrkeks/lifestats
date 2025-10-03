use crate::plot::plot_config::PlotConfig;
use crate::plot::plot_metadata::PlotMetadata;
use crate::plot::traits::plot::Plot;
use crate::plot::types::bar::BarPlot;
use crate::plot::types::circular_progress_bar::CircularProgressBarPlot;
use crate::plot::types::scatter::ScatterPlot;
use crate::plot::types::time_frames::TimeFramesPlot;
use anyhow::Result;
use transfer_objects::image::Image;
use transfer_objects::statistic_id::{PlotId, StatisticId};

pub enum AnyPlot {
    Bar(BarPlot),
    CircularProgressBar(CircularProgressBarPlot),
    Scatter(ScatterPlot),
    TimeFrames(TimeFramesPlot),
    // TimeLine(TimeLinePlot),
}

impl AnyPlot {
    pub fn get_width(&self) -> usize {
        match self {
            AnyPlot::Bar(plot) => plot.get_width(),
            AnyPlot::CircularProgressBar(plot) => plot.get_width(),
            AnyPlot::Scatter(plot) => plot.get_width(),
            AnyPlot::TimeFrames(plot) => plot.get_width(),
            // AnyPlot::TimeLine(plot) => plot.get_width(),
        }
    }

    pub fn get_height(&self) -> usize {
        match self {
            AnyPlot::Bar(plot) => plot.get_height(),
            AnyPlot::CircularProgressBar(plot) => plot.get_height(),
            AnyPlot::Scatter(plot) => plot.get_height(),
            AnyPlot::TimeFrames(plot) => plot.get_height(),
            // AnyPlot::TimeLine(plot) => plot.get_height(),
        }
    }

    pub fn plot(&self, plot_config: PlotConfig) -> Result<Image> {
        match self {
            AnyPlot::Bar(plot) => plot.plot(plot_config),
            AnyPlot::CircularProgressBar(plot) => plot.plot(plot_config),
            AnyPlot::Scatter(plot) => plot.plot(plot_config),
            AnyPlot::TimeFrames(plot) => plot.plot(plot_config),
        }
    }

    pub fn get_metadata(&self) -> &PlotMetadata {
        match self {
            AnyPlot::Bar(plot) => plot.get_metadata(),
            AnyPlot::CircularProgressBar(plot) => plot.get_metadata(),
            // ... other plot types
            _ => unimplemented!(), // or handle other plot types
        }
    }

    pub fn contains_statistic_with_id(&self, id: StatisticId) -> bool {
        match self {
            AnyPlot::Bar(plot) => plot.contains_statistic_with_id(id),
            // ... other plot types
            _ => unimplemented!(), // or handle other plot types
        }
    }

    pub fn get_statistic_id(&self) -> StatisticId {
        match self {
            AnyPlot::Bar(plot) => plot.get_statistic_id(),
            AnyPlot::CircularProgressBar(plot) => plot.get_statistic_id(),
            AnyPlot::Scatter(plot) => plot.get_statistic_id(),
            AnyPlot::TimeFrames(plot) => plot.get_statistic_id(),
            // AnyPlot::TimeLine(plot) => plot.get_statistic_id(),
        }
    }

    pub fn get_id(&self) -> PlotId {
        match self {
            AnyPlot::Bar(plot) => plot.get_id(),
            AnyPlot::CircularProgressBar(plot) => plot.get_id(),
            AnyPlot::Scatter(plot) => plot.get_id(),
            AnyPlot::TimeFrames(plot) => plot.get_id(),
            // AnyPlot::TimeLine(plot) => plot.get_id(),
        }
    }

    pub fn get_title(&self) -> &str {
        match self {
            AnyPlot::Bar(plot) => plot.get_title(),
            AnyPlot::CircularProgressBar(plot) => plot.get_title(),
            AnyPlot::Scatter(plot) => plot.get_title(),
            AnyPlot::TimeFrames(plot) => plot.get_title(),
            // AnyPlot::TimeLine(plot) => plot.get_title(),
        }
    }
}
