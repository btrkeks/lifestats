use crate::plot::objects::color::Color;
use crate::plot::objects::plot_data::PlotDataPoint;
use crate::plot::objects::y_axis_conversion::YAxisConversion;
use crate::plot::plot_backends::plotters::barplot::barplot;
use crate::plot::plot_backends::plotters::circular_progress_plot::circular_progress_plot;
use crate::plot::plot_backends::plotters::common::color_to_plotters;
use crate::plot::plot_backends::plotters::scatterplot::scatter_plot;
use crate::plot::plot_backends::plotters::time_frames_plot::time_frames_plot;
use crate::plot::plot_backends::plotting_backend::PlottingBackend;
use crate::statistic::objects::time_frame_list::TimeFrameList;
use anyhow::Result;
use chrono::NaiveDate;
use transfer_objects::image::Image;

mod barplot;
mod circular_progress_plot;
mod common;
mod constants;
mod scatterplot;
mod time_frames_plot;
mod time_line_plot;

#[derive(Clone, Debug)]
pub struct PlottersBackend;

impl PlottingBackend for PlottersBackend {
    fn bar_plot(
        &self,
        title: &str,
        size: (usize, usize),
        data: Vec<PlotDataPoint<i64>>,
        y_unit: &str,
        border_color: Color,
        y_axis_conversion: YAxisConversion,
        x_min: NaiveDate,
        x_max: NaiveDate,
        streak: Option<u32>,
    ) -> Result<Image> {
        barplot(
            title,
            size,
            data,
            y_unit,
            color_to_plotters(&border_color),
            y_axis_conversion,
            x_min,
            x_max,
            streak,
        )
    }

    fn scatter_plot(
        &self,
        title: String,
        size: (usize, usize),
        data: Vec<PlotDataPoint<i64>>,
        y_unit: String,
        border_color: Color,
        y_axis_conversion: YAxisConversion,
        x_min: NaiveDate,
        x_max: NaiveDate,
        streak: Option<u32>,
    ) -> Result<Image> {
        scatter_plot(
            title,
            size,
            data,
            y_unit,
            color_to_plotters(&border_color),
            y_axis_conversion,
            x_min,
            x_max,
            streak,
        )
    }

    fn time_frames_plot(
        &self,
        title: String,
        size: (usize, usize),
        data: Vec<PlotDataPoint<TimeFrameList>>,
        y_unit: String,
        border_color: Color,
        x_min: NaiveDate,
        x_max: NaiveDate,
        streak: Option<u32>,
    ) -> Result<Image> {
        time_frames_plot(
            title,
            size,
            data,
            y_unit,
            color_to_plotters(&border_color),
            x_min,
            x_max,
            streak,
        )
    }

    fn circular_progress_plot(
        &self,
        title: String,
        size: (usize, usize),
        value: i64,
        max_value: i64,
        color: Color,
    ) -> Result<Image> {
        circular_progress_plot(title, size, value, max_value, color)
    }
}
