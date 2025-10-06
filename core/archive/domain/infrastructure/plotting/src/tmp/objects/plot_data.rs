use crate::plot::objects::color::Color;
use chrono::NaiveDate;

pub struct PlotDataPoint<T> {
    pub date: NaiveDate,
    pub value: Option<T>,
    pub goal: Option<T>,
    pub color: Color,
}

impl<T> PlotDataPoint<T> {
    pub fn new(date: NaiveDate, value: Option<T>, goal: Option<T>, color: Color) -> Self {
        Self {
            date,
            value,
            goal,
            color,
        }
    }
}
