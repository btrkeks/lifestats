use chrono::{NaiveDate, NaiveTime};
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::time_frame_list::TimeFrameList;
use plot_domain::plot::axis::YAxis;
use plot_domain::plot::data::{DataPoint, DrawableData, DrawableGoal, PlotData};
use plot_domain::plot::plot_type::PlotType;
use serde::{Deserialize, Serialize};

/// Response DTO for plot data
#[derive(Debug, Serialize, Deserialize)]
pub struct PlotDataResponse {
    pub title: String,
    pub points: Vec<DataPointResponse>,
    pub date_range: DateRangeResponse,
    pub plot_type: PlotType,
    pub y_axis: YAxisResponse,
}

impl From<PlotData> for PlotDataResponse {
    fn from(plot_data: PlotData) -> Self {
        Self {
            title: plot_data.title,
            points: plot_data.points.into_iter().map(Into::into).collect(),
            date_range: plot_data.date_range.into(),
            plot_type: plot_data.plot_type,
            y_axis: plot_data.y_axis.into(),
        }
    }
}

/// Response DTO for a data point
#[derive(Debug, Serialize, Deserialize)]
pub struct DataPointResponse {
    pub date: NaiveDate,
    pub value: DrawableDataResponse,
    pub color: [u8; 3],
    pub goal: Option<DrawableGoalResponse>,
}

impl From<DataPoint> for DataPointResponse {
    fn from(point: DataPoint) -> Self {
        Self {
            date: point.date,
            value: point.value.into(),
            color: point.color,
            goal: point.goal.map(Into::into),
        }
    }
}

/// Response DTO for drawable data
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum DrawableDataResponse {
    Integer(i64),
    TimeFrames(TimeFrameList),
}

impl From<DrawableData> for DrawableDataResponse {
    fn from(data: DrawableData) -> Self {
        match data {
            DrawableData::Integer(val) => DrawableDataResponse::Integer(val),
            DrawableData::TimeFrames(frames) => DrawableDataResponse::TimeFrames(frames),
        }
    }
}

/// Response DTO for drawable goal
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum DrawableGoalResponse {
    Integer(i64),
    TimeFrames(TimeFrameList),
}

impl From<DrawableGoal> for DrawableGoalResponse {
    fn from(goal: DrawableGoal) -> Self {
        match goal {
            DrawableGoal::Integer(val) => DrawableGoalResponse::Integer(val),
            DrawableGoal::TimeFrames(frames) => DrawableGoalResponse::TimeFrames(frames),
        }
    }
}

/// Response DTO for Y-axis configuration
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum YAxisResponse {
    Integer { min: i64, max: i64 },
    Time { min: NaiveTime, max: NaiveTime },
}

impl From<YAxis> for YAxisResponse {
    fn from(axis: YAxis) -> Self {
        match axis {
            YAxis::Integer { min, max } => YAxisResponse::Integer { min, max },
            YAxis::Time { min, max } => YAxisResponse::Time { min, max },
        }
    }
}

/// Response DTO for date range
#[derive(Debug, Serialize, Deserialize)]
pub struct DateRangeResponse {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl From<DateRange> for DateRangeResponse {
    fn from(range: DateRange) -> Self {
        Self {
            start: range.start(),
            end: range.end(),
        }
    }
}
