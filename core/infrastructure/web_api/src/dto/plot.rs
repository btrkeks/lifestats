use chrono::NaiveDate;
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::time_frame_list::TimeFrameList;
use plot_domain::plot::axis::{AxisLabelFormat, YAxis};
use plot_domain::plot::data::{
    DataPoint, DrawableData, DrawableGoal, PlotData, ScalarUnit, ScalarValue,
};
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
    pub value: Option<DrawableDataResponse>,
    pub color: [u8; 3],
    pub goal: Option<DrawableGoalResponse>,
}

impl From<DataPoint> for DataPointResponse {
    fn from(point: DataPoint) -> Self {
        Self {
            date: point.date,
            value: point.value.map(Into::into),
            color: point.color,
            goal: point.goal.map(Into::into),
        }
    }
}

/// Response DTO for drawable data
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum DrawableDataResponse {
    Scalar(ScalarValueResponse),
    TimeFrames(TimeFrameList),
}

impl From<DrawableData> for DrawableDataResponse {
    fn from(data: DrawableData) -> Self {
        match data {
            DrawableData::Scalar(val) => DrawableDataResponse::Scalar(val.into()),
            DrawableData::TimeFrames(frames) => DrawableDataResponse::TimeFrames(frames),
        }
    }
}

/// Response DTO for drawable goal
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum DrawableGoalResponse {
    Scalar(ScalarValueResponse),
    TimeFrames(TimeFrameList),
}

impl From<DrawableGoal> for DrawableGoalResponse {
    fn from(goal: DrawableGoal) -> Self {
        match goal {
            DrawableGoal::Scalar(val) => DrawableGoalResponse::Scalar(val.into()),
            DrawableGoal::TimeFrames(frames) => DrawableGoalResponse::TimeFrames(frames),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScalarValueResponse {
    pub value: i64,
    pub unit: String,
}

impl From<ScalarValue> for ScalarValueResponse {
    fn from(value: ScalarValue) -> Self {
        Self {
            value: value.value(),
            unit: scalar_unit_to_string(value.unit()).to_string(),
        }
    }
}

/// Response DTO for Y-axis configuration
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum YAxisResponse {
    Scalar {
        min: i64,
        max: i64,
        label_format: String,
    },
    TimeOfDay {
        min_seconds: i64,
        max_seconds: i64,
        label_format: String,
    },
}

impl From<YAxis> for YAxisResponse {
    fn from(axis: YAxis) -> Self {
        match axis {
            YAxis::Scalar {
                min,
                max,
                label_format,
            } => YAxisResponse::Scalar {
                min,
                max,
                label_format: axis_label_format_to_string(label_format).to_string(),
            },
            YAxis::TimeOfDay {
                min_seconds,
                max_seconds,
                label_format,
            } => YAxisResponse::TimeOfDay {
                min_seconds,
                max_seconds,
                label_format: axis_label_format_to_string(label_format).to_string(),
            },
        }
    }
}

fn scalar_unit_to_string(unit: ScalarUnit) -> &'static str {
    match unit {
        ScalarUnit::Number => "number",
        ScalarUnit::Binary => "binary",
        ScalarUnit::DurationSeconds => "duration_seconds",
        ScalarUnit::TimeOfDaySeconds => "time_of_day_seconds",
        ScalarUnit::Percent => "percent",
    }
}

fn axis_label_format_to_string(format: AxisLabelFormat) -> &'static str {
    match format {
        AxisLabelFormat::Number => "number",
        AxisLabelFormat::DurationHhMm => "duration_hh_mm",
        AxisLabelFormat::TimeOfDayHhMm => "time_of_day_hh_mm",
        AxisLabelFormat::Percent => "percent",
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
