use crate::plot::objects::plot_metadata::PlotMetadata;
use crate::plot::plot::Plot;
use crate::plot::plot_types::bar::BarPlot;
use crate::plot::plot_types::circular_progress_bar::CircularProgressBarPlot;
use crate::plot::plot_types::scatter::ScatterPlot;
use crate::plot::plot_types::time_frames::TimeFramesPlot;
use anyhow::Result;
use chrono::NaiveDate;
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

    pub fn plot(&self, start_date: NaiveDate, end_date: NaiveDate) -> Result<Image> {
        match self {
            AnyPlot::Bar(plot) => plot.plot(start_date, end_date),
            AnyPlot::CircularProgressBar(plot) => plot.plot(start_date, end_date),
            AnyPlot::Scatter(plot) => plot.plot(start_date, end_date),
            AnyPlot::TimeFrames(plot) => plot.plot(start_date, end_date),
            // AnyPlot::TimeLine(plot) => plot.plot(size, start_date, end_date),
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
