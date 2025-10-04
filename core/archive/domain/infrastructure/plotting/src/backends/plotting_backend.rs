use crate::plot::integer_plot_data::IntegerPlotData;
use crate::plot::objects::color::Color;
use crate::plot::objects::plot_data::PlotDataPoint;
use crate::value_objects::time_frame_list::TimeFrameList;
use anyhow::Result;
use chrono::NaiveDate;
use transfer_objects::image::Image;

pub trait PlottingBackend {
    fn bar_plot(
        &self,
        plot_data: IntegerPlotData
    ) -> Result<Image>;

    fn scatter_plot(
        &self,
        plot_data: IntegerPlotData
    ) -> Result<Image>;

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
    ) -> Result<Image>;

    fn circular_progress_plot(
        &self,
        title: String,
        size: (usize, usize),
        value: i64,
        max_value: i64,
        color: Color,
    ) -> Result<Image>;
}
