use crate::util::axis_label_formatter::{
    int_to_string_formatter, seconds_to_time_formatter,
};
use backend::statistics::objects::{StatisticId, TimeFrame, TimeFrameList};
use chrono::{NaiveDate, NaiveTime};
use plotters::prelude::RGBColor;
use crate::util::tick_calculation::{calculate_int_ticks, calculate_seconds_ticks};

#[derive(Debug, PartialOrd, PartialEq)]
pub enum YValueFormatting {
    SecondsToTime,
    None,
}

pub struct BarplotDataPoint {
    pub date: NaiveDate,
    pub value: i64,
    pub goal: Option<i64>,
    pub color: RGBColor,
}

pub struct BarplotData {
    pub y_value_formatting: YValueFormatting,
    pub border_color: RGBColor,
    pub data_points: Vec<BarplotDataPoint>,
}

impl BarplotData {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            y_value_formatting: YValueFormatting::None,
            border_color: RGBColor(0, 0, 0),
            data_points: Vec::with_capacity(capacity),
        }
    }

    pub fn get_y_formatter(&self) -> fn(&i64) -> String {
        match self.y_value_formatting {
            YValueFormatting::SecondsToTime => seconds_to_time_formatter,
            YValueFormatting::None => int_to_string_formatter,
        }
    }

    pub fn get_y_ticks(&self, y_axis_min: i64, y_axis_max: i64, plot_height: u32) -> Vec<i64> {
        match self.y_value_formatting {
            YValueFormatting::SecondsToTime => {
                calculate_seconds_ticks(y_axis_min, y_axis_max, plot_height)
            }
            YValueFormatting::None => calculate_int_ticks(y_axis_min, y_axis_max, plot_height),
        }
    }
}

pub struct TimeFramesPlotPoint {
    pub date: NaiveDate,
    pub time_frames: TimeFrameList,
    pub goal_time_frames: TimeFrameList,
    pub color: RGBColor,
}

pub struct TimeFramesPlotData {
    pub border_color: RGBColor,
    pub data_points: Vec<TimeFramesPlotPoint>,
}

pub struct LabeledTimeFrame {
    pub stat_id: StatisticId,
    pub label: String,
    pub time_frame: TimeFrame,
    pub color: Option<RGBColor>,
}

impl LabeledTimeFrame {
    pub fn from_time_frame(
        stat_id: StatisticId,
        label: String,
        time_frame: TimeFrame,
        color: Option<RGBColor>,
    ) -> LabeledTimeFrame {
        LabeledTimeFrame {
            stat_id,
            label,
            time_frame,
            color,
        }
    }
}

pub struct LabeledTime {
    pub label: String,
    pub time: NaiveTime,
}

pub struct TimelineData {
    pub(crate) time_frames: Vec<LabeledTimeFrame>,
    pub(crate) time_points: Vec<LabeledTime>,
}

pub struct TimePointPlotPoint {
    pub(crate) date: NaiveDate,
    pub(crate) time: NaiveTime,
    pub(crate) goal: Option<NaiveTime>,
    pub(crate) color: RGBColor,
}

pub struct TimePointPlotData {
    pub(crate) border_color: RGBColor,
    pub(crate) data_points: Vec<TimePointPlotPoint>,
}