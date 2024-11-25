use crate::backends::plotters::barplot::barplot;
use crate::backends::plotting_backend::PlottingBackend;
use crate::objects::color::Color;
use crate::objects::image::Image;
use crate::objects::plot_data::{PlotDataPoint};
use anyhow::Result;
use chrono::NaiveDate;
use crate::backends::plotters::common::color_to_plotters;
use crate::objects::y_axis_conversion::YAxisConversion;

pub struct PlottersBackend;

impl PlottingBackend for PlottersBackend {
    fn barplot(&self, title: String,
               size: (usize, usize),
               data: Vec<PlotDataPoint<i64>>,
               y_unit: String,
               border_color: Color,
               y_axis_conversion: YAxisConversion,
               x_min: NaiveDate,
               x_max: NaiveDate,
    ) -> Result<Image> {
        barplot(title,
                size,
                data,
                y_unit,
                color_to_plotters(border_color),
                y_axis_conversion,
                x_min,
                x_max,
        )
    }
}