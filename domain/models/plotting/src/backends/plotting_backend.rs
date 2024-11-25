use crate::objects::color::Color;
use crate::objects::image::Image;
use crate::objects::plot_data::{PlotDataPoint};
use anyhow::Result;
use chrono::NaiveDate;
use crate::objects::y_axis_conversion::YAxisConversion;

pub trait PlottingBackend {
    fn barplot(&self, title: String,
               size: (usize, usize),
               data: Vec<PlotDataPoint<i64>>,
               y_unit: String,
               border_color: Color,
               y_axis_conversion: YAxisConversion,
               x_min: NaiveDate,
               x_max: NaiveDate,
    ) -> Result<Image>;
}