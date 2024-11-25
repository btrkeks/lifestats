use chrono::NaiveDate;
use crate::objects::color::Color;
use crate::plot::Plot;

pub struct PlotDataPoint<T> {
    pub date: NaiveDate,
    pub value: T,
    pub goal: Option<T>,
    pub color: Color,
}