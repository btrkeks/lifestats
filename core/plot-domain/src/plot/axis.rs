use crate::plot::data::{DataPoint, DrawableData, DrawableGoal, ScalarUnit};
use crate::plot::plot_type::PlotType;
use chrono::Timelike;

const SECONDS_PER_DAY: i64 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisLabelFormat {
    Number,
    DurationHhMm,
    TimeOfDayHhMm,
    Percent,
}

/// Represents a configured Y-axis with range and label semantics.
#[derive(Debug, Clone, PartialEq)]
pub enum YAxis {
    Scalar {
        min: i64,
        max: i64,
        label_format: AxisLabelFormat,
    },
    TimeOfDay {
        min_seconds: i64,
        max_seconds: i64,
        label_format: AxisLabelFormat,
    },
}

impl YAxis {
    pub fn from_data_points(points: &[DataPoint], plot_type: PlotType) -> anyhow::Result<Self> {
        match plot_type {
            PlotType::BarChart | PlotType::LineChart | PlotType::Scatter => {
                Self::calculate_scalar_axis(points)
            }
            PlotType::TimeFrames => Self::calculate_time_frames_axis(points),
            PlotType::CircularProgress => Ok(Self::Scalar {
                min: 0,
                max: 100,
                label_format: AxisLabelFormat::Percent,
            }),
        }
    }

    fn calculate_scalar_axis(points: &[DataPoint]) -> anyhow::Result<Self> {
        let mut values = Vec::new();
        let mut unit = None;

        for point in points {
            if let Some(DrawableData::Scalar(value)) = &point.value {
                values.push(value.value());
                unit = Some(resolve_unit(unit, value.unit()));
            }

            if let Some(DrawableGoal::Scalar(value)) = &point.goal {
                values.push(value.value());
                unit = Some(resolve_unit(unit, value.unit()));
            }
        }

        let unit = unit.unwrap_or(ScalarUnit::Number);
        if values.is_empty() {
            return Ok(default_scalar_axis(unit));
        }

        let min = *values.iter().min().unwrap();
        let max = *values.iter().max().unwrap();
        Ok(axis_for_scalar_range(min, max, unit))
    }

    fn calculate_time_frames_axis(points: &[DataPoint]) -> anyhow::Result<Self> {
        let mut all_seconds = Vec::new();

        for point in points {
            if let Some(DrawableData::TimeFrames(time_frame_list)) = &point.value {
                if let Some(start) = time_frame_list.start_time() {
                    all_seconds.push(start.num_seconds_from_midnight() as i64);
                }
                if let Some(end) = time_frame_list.end_time() {
                    all_seconds.push(end.num_seconds_from_midnight() as i64);
                }
            }

            if let Some(DrawableGoal::TimeFrames(time_frame_list)) = &point.goal {
                if let Some(start) = time_frame_list.start_time() {
                    all_seconds.push(start.num_seconds_from_midnight() as i64);
                }
                if let Some(end) = time_frame_list.end_time() {
                    all_seconds.push(end.num_seconds_from_midnight() as i64);
                }
            }
        }

        if all_seconds.is_empty() {
            return Ok(Self::TimeOfDay {
                min_seconds: 0,
                max_seconds: SECONDS_PER_DAY - 1,
                label_format: AxisLabelFormat::TimeOfDayHhMm,
            });
        }

        let min_seconds = *all_seconds.iter().min().unwrap();
        let max_seconds = *all_seconds.iter().max().unwrap();

        Ok(Self::TimeOfDay {
            min_seconds,
            max_seconds,
            label_format: AxisLabelFormat::TimeOfDayHhMm,
        })
    }
}

fn resolve_unit(current: Option<ScalarUnit>, next: ScalarUnit) -> ScalarUnit {
    match (current, next) {
        (None, unit) => unit,
        (Some(unit), next) if unit == next => unit,
        (Some(ScalarUnit::DurationSeconds), ScalarUnit::Number)
        | (Some(ScalarUnit::Number), ScalarUnit::DurationSeconds) => ScalarUnit::DurationSeconds,
        (Some(ScalarUnit::TimeOfDaySeconds), ScalarUnit::Number)
        | (Some(ScalarUnit::Number), ScalarUnit::TimeOfDaySeconds) => ScalarUnit::TimeOfDaySeconds,
        (Some(unit), _) => unit,
    }
}

fn default_scalar_axis(unit: ScalarUnit) -> YAxis {
    match unit {
        ScalarUnit::DurationSeconds => YAxis::Scalar {
            min: 0,
            max: 3600,
            label_format: AxisLabelFormat::DurationHhMm,
        },
        ScalarUnit::TimeOfDaySeconds => YAxis::TimeOfDay {
            min_seconds: 0,
            max_seconds: SECONDS_PER_DAY - 1,
            label_format: AxisLabelFormat::TimeOfDayHhMm,
        },
        ScalarUnit::Percent => YAxis::Scalar {
            min: 0,
            max: 100,
            label_format: AxisLabelFormat::Percent,
        },
        ScalarUnit::Number => YAxis::Scalar {
            min: 0,
            max: 100,
            label_format: AxisLabelFormat::Number,
        },
    }
}

fn axis_for_scalar_range(min: i64, max: i64, unit: ScalarUnit) -> YAxis {
    let (mut padded_min, mut padded_max) = padded_range(min, max);

    match unit {
        ScalarUnit::DurationSeconds => {
            padded_min = 0;
            padded_max = padded_max.max(1);
            YAxis::Scalar {
                min: padded_min,
                max: padded_max,
                label_format: AxisLabelFormat::DurationHhMm,
            }
        }
        ScalarUnit::TimeOfDaySeconds => {
            padded_min = padded_min.clamp(0, SECONDS_PER_DAY - 1);
            padded_max = padded_max.clamp(0, SECONDS_PER_DAY - 1);
            if padded_min == padded_max {
                padded_max = (padded_max + 3600).min(SECONDS_PER_DAY - 1);
                padded_min = padded_min.saturating_sub(3600);
            }
            YAxis::TimeOfDay {
                min_seconds: padded_min,
                max_seconds: padded_max,
                label_format: AxisLabelFormat::TimeOfDayHhMm,
            }
        }
        ScalarUnit::Percent => YAxis::Scalar {
            min: padded_min.max(0),
            max: padded_max.max(100),
            label_format: AxisLabelFormat::Percent,
        },
        ScalarUnit::Number => YAxis::Scalar {
            min: padded_min,
            max: padded_max,
            label_format: AxisLabelFormat::Number,
        },
    }
}

fn padded_range(min: i64, max: i64) -> (i64, i64) {
    let range = max - min;
    let padding = std::cmp::max((range as f64 * 0.1) as i64, 5);
    (min - padding, max + padding)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plot::data::ScalarValue;
    use chrono::NaiveDate;
    use serde_json::json;

    fn scalar(value: i64, unit: ScalarUnit) -> DrawableData {
        DrawableData::Scalar(ScalarValue::new(value, unit))
    }

    fn scalar_goal(value: i64, unit: ScalarUnit) -> DrawableGoal {
        DrawableGoal::Scalar(ScalarValue::new(value, unit))
    }

    #[test]
    fn scalar_number_axis_with_values() {
        let points = vec![
            DataPoint::new(
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                Some(scalar(10, ScalarUnit::Number)),
                [0, 255, 0],
                Some(scalar_goal(20, ScalarUnit::Number)),
            ),
            DataPoint::new(
                NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
                Some(scalar(50, ScalarUnit::Number)),
                [0, 255, 0],
                None,
            ),
        ];

        let axis = YAxis::from_data_points(&points, PlotType::BarChart).unwrap();

        match axis {
            YAxis::Scalar {
                min,
                max,
                label_format,
            } => {
                assert_eq!(label_format, AxisLabelFormat::Number);
                assert!(min < 10);
                assert!(max > 50);
            }
            _ => panic!("Expected scalar axis"),
        }
    }

    #[test]
    fn duration_axis_uses_duration_labels_and_clamps_to_zero() {
        let points = vec![DataPoint::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            Some(scalar(3600, ScalarUnit::DurationSeconds)),
            [0, 255, 0],
            Some(scalar_goal(7200, ScalarUnit::DurationSeconds)),
        )];

        let axis = YAxis::from_data_points(&points, PlotType::LineChart).unwrap();

        assert_eq!(
            axis,
            YAxis::Scalar {
                min: 0,
                max: 7560,
                label_format: AxisLabelFormat::DurationHhMm,
            }
        );
    }

    #[test]
    fn time_of_day_axis_uses_clock_labels() {
        let points = vec![DataPoint::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            Some(scalar(9 * 3600, ScalarUnit::TimeOfDaySeconds)),
            [0, 255, 0],
            Some(scalar_goal(17 * 3600, ScalarUnit::TimeOfDaySeconds)),
        )];

        let axis = YAxis::from_data_points(&points, PlotType::Scatter).unwrap();

        match axis {
            YAxis::TimeOfDay {
                min_seconds,
                max_seconds,
                label_format,
            } => {
                assert_eq!(label_format, AxisLabelFormat::TimeOfDayHhMm);
                assert!(min_seconds < 9 * 3600);
                assert!(max_seconds > 17 * 3600);
            }
            _ => panic!("Expected time-of-day axis"),
        }
    }

    #[test]
    fn scalar_axis_empty_data_defaults_to_number_axis() {
        let axis = YAxis::from_data_points(&[], PlotType::BarChart).unwrap();

        assert_eq!(
            axis,
            YAxis::Scalar {
                min: 0,
                max: 100,
                label_format: AxisLabelFormat::Number,
            }
        );
    }

    #[test]
    fn time_frames_axis_with_time_frames() {
        use lifestats_core::shared::time_frame_list::TimeFrameList;

        let time_frame_list = serde_json::from_value::<TimeFrameList>(json!([
            {"start": "09:00:00", "end": "12:00:00"},
            {"start": "14:00:00", "end": "18:00:00"}
        ]))
        .unwrap();

        let points = vec![DataPoint::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            Some(DrawableData::TimeFrames(time_frame_list)),
            [0, 255, 0],
            None,
        )];

        let axis = YAxis::from_data_points(&points, PlotType::TimeFrames).unwrap();

        assert_eq!(
            axis,
            YAxis::TimeOfDay {
                min_seconds: 9 * 3600,
                max_seconds: 18 * 3600,
                label_format: AxisLabelFormat::TimeOfDayHhMm,
            }
        );
    }

    #[test]
    fn time_frames_axis_empty_data_defaults_to_full_day() {
        let axis = YAxis::from_data_points(&[], PlotType::TimeFrames).unwrap();

        assert_eq!(
            axis,
            YAxis::TimeOfDay {
                min_seconds: 0,
                max_seconds: SECONDS_PER_DAY - 1,
                label_format: AxisLabelFormat::TimeOfDayHhMm,
            }
        );
    }

    #[test]
    fn circular_progress_default_range() {
        let axis = YAxis::from_data_points(&[], PlotType::CircularProgress).unwrap();

        assert_eq!(
            axis,
            YAxis::Scalar {
                min: 0,
                max: 100,
                label_format: AxisLabelFormat::Percent,
            }
        );
    }
}
