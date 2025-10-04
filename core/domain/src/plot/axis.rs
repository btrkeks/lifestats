use crate::plot::data::{DataPoint, DrawableData};
use crate::plot::specification::PlotType;
use chrono::NaiveTime;

/// Represents a configured Y-axis with type and range
/// This is a domain concept that encapsulates the Y-axis calculation logic
#[derive(Debug, Clone, PartialEq)]
pub enum YAxis {
    Integer { min: i64, max: i64 },
    Time { min: NaiveTime, max: NaiveTime },
}

impl YAxis {
    /// Creates a YAxis configuration from data points and plot type
    /// This is pure domain logic that determines the appropriate axis range
    pub fn from_data_points(points: &[DataPoint], plot_type: PlotType) -> anyhow::Result<Self> {
        match plot_type {
            PlotType::BarChart | PlotType::LineChart | PlotType::Scatter => {
                Self::calculate_integer_axis(points)
            }
            PlotType::TimeFrames => Self::calculate_time_axis(points),
            PlotType::CircularProgress => {
                // Circular progress has an implicit 0-100 range
                Ok(Self::Integer { min: 0, max: 100 })
            }
        }
    }

    fn calculate_integer_axis(points: &[DataPoint]) -> anyhow::Result<Self> {
        let mut values: Vec<i64> = Vec::new();

        // Collect all numeric values from both value and goal fields
        for point in points {
            // Use pattern matching on DrawableData enum - type-safe!
            if let DrawableData::Integer(val) = &point.value {
                values.push(*val);
            }

            // Goal is still JSON for reference purposes
            if let Some(goal) = &point.goal {
                if let Some(val) = goal.as_i64() {
                    values.push(val);
                } else if let Some(val) = goal.as_f64() {
                    values.push(val as i64);
                }
            }
        }

        if values.is_empty() {
            // Default to a sensible range when no data is available
            return Ok(Self::Integer { min: 0, max: 100 });
        }

        let min = *values.iter().min().unwrap();
        let max = *values.iter().max().unwrap();

        // Calculate padding (10% of range, with minimum of 5)
        let range = max - min;
        let padding = std::cmp::max((range as f64 * 0.1) as i64, 5);

        Ok(Self::Integer {
            min: min - padding,
            max: max + padding,
        })
    }

    fn calculate_time_axis(points: &[DataPoint]) -> anyhow::Result<Self> {
        let mut all_times: Vec<NaiveTime> = Vec::new();

        // Collect all start and end times from TimeFrameLists
        for point in points {
            // Use pattern matching on DrawableData enum - type-safe!
            if let DrawableData::TimeFrames(time_frame_list) = &point.value {
                if let Some(start) = time_frame_list.start_time() {
                    all_times.push(start);
                }
                if let Some(end) = time_frame_list.end_time() {
                    all_times.push(end);
                }
            }

            // Goal is still JSON for reference purposes
            if let Some(goal) = &point.goal {
                if let Ok(time_frame_list) = serde_json::from_value::<
                    crate::shared::time_frame_list::TimeFrameList,
                >(goal.clone())
                {
                    if let Some(start) = time_frame_list.start_time() {
                        all_times.push(start);
                    }
                    if let Some(end) = time_frame_list.end_time() {
                        all_times.push(end);
                    }
                }
            }
        }

        if all_times.is_empty() {
            // Default to full day range
            return Ok(Self::Time {
                min: NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                max: NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
            });
        }

        let min = *all_times.iter().min().unwrap();
        let max = *all_times.iter().max().unwrap();

        Ok(Self::Time { min, max })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::json;

    #[test]
    fn test_integer_axis_with_values() {
        let points = vec![
            DataPoint::new(
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                DrawableData::Integer(10),
                [0, 255, 0],
                Some(json!(20)),
            ),
            DataPoint::new(
                NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
                DrawableData::Integer(30),
                [0, 255, 0],
                Some(json!(40)),
            ),
            DataPoint::new(
                NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
                DrawableData::Integer(50),
                [0, 255, 0],
                None,
            ),
        ];

        let axis = YAxis::from_data_points(&points, PlotType::BarChart).unwrap();

        match axis {
            YAxis::Integer { min, max } => {
                assert!(min < 10, "Min should be padded below lowest value (10)");
                assert!(max > 50, "Max should be padded above highest value (50)");
            }
            _ => panic!("Expected Integer axis"),
        }
    }

    #[test]
    fn test_integer_axis_empty_data() {
        let points = vec![];
        let axis = YAxis::from_data_points(&points, PlotType::LineChart).unwrap();

        assert_eq!(axis, YAxis::Integer { min: 0, max: 100 });
    }

    #[test]
    fn test_time_axis_with_time_frames() {
        use crate::shared::time_frame_list::TimeFrameList;

        let time_frame_list = serde_json::from_value::<TimeFrameList>(json!([
            {"start": "09:00:00", "end": "12:00:00"},
            {"start": "14:00:00", "end": "18:00:00"}
        ]))
        .unwrap();

        let points = vec![DataPoint::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            DrawableData::TimeFrames(time_frame_list),
            [0, 255, 0],
            None,
        )];

        let axis = YAxis::from_data_points(&points, PlotType::TimeFrames).unwrap();

        match axis {
            YAxis::Time { min, max } => {
                assert_eq!(min, NaiveTime::from_hms_opt(9, 0, 0).unwrap());
                assert_eq!(max, NaiveTime::from_hms_opt(18, 0, 0).unwrap());
            }
            _ => panic!("Expected Time axis"),
        }
    }

    #[test]
    fn test_time_axis_empty_data() {
        let points = vec![];
        let axis = YAxis::from_data_points(&points, PlotType::TimeFrames).unwrap();

        assert_eq!(
            axis,
            YAxis::Time {
                min: NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                max: NaiveTime::from_hms_opt(23, 59, 59).unwrap()
            }
        );
    }

    #[test]
    fn test_circular_progress_default_range() {
        let points = vec![DataPoint::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            DrawableData::Integer(75),
            [0, 255, 0],
            Some(json!(100)),
        )];

        let axis = YAxis::from_data_points(&points, PlotType::CircularProgress).unwrap();

        assert_eq!(axis, YAxis::Integer { min: 0, max: 100 });
    }

    #[test]
    fn test_integer_axis_with_floats() {
        // Note: DrawableData uses i64, so we store the truncated values
        let points = vec![
            DataPoint::new(
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                DrawableData::Integer(10),
                [0, 255, 0],
                None,
            ),
            DataPoint::new(
                NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
                DrawableData::Integer(20),
                [0, 255, 0],
                None,
            ),
        ];

        let axis = YAxis::from_data_points(&points, PlotType::BarChart).unwrap();

        match axis {
            YAxis::Integer { min, max } => {
                assert!(min <= 10);
                assert!(max >= 20);
            }
            _ => panic!("Expected Integer axis"),
        }
    }

    #[test]
    fn test_integer_axis_single_value() {
        let points = vec![DataPoint::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            DrawableData::Integer(42),
            [0, 255, 0],
            None,
        )];

        let axis = YAxis::from_data_points(&points, PlotType::Scatter).unwrap();

        match axis {
            YAxis::Integer { min, max } => {
                // When all values are the same, we should still have padding
                assert!(min < 42);
                assert!(max > 42);
                assert_eq!(max - min, 10); // Minimum padding is 5 on each side
            }
            _ => panic!("Expected Integer axis"),
        }
    }
}
