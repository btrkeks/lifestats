/// Service for converting JSON values to drawable plot data types
///
/// This module contains the business logic for transforming stored values (JSON)
/// into strongly-typed, renderer-ready data (DrawableData/DrawableGoal).
/// This conversion logic belongs in the application layer, not the web API adapter.
use anyhow::{anyhow, Result};
use chrono::Timelike;
use lifestats_core::shared::percentage::Percentage;
use lifestats_core::shared::time_frame_list::TimeFrameList;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use plot_domain::plot::data::{DrawableData, DrawableGoal};
use serde_json::Value;

/// Converts a JSON log value to DrawableData based on the statistic type and plot type
///
/// # Arguments
/// * `statistic` - The statistic metadata to determine the expected type
/// * `value` - The JSON value from the database
/// * `plot_type` - The type of plot being rendered (affects conversion for TimeSpans)
///
/// # Returns
/// * `Ok(DrawableData)` - The typed drawable data
/// * `Err` - If the JSON structure doesn't match the expected statistic type
pub fn json_value_to_drawable_data(
    statistic: &AnyStatistic,
    value: &Value,
    plot_type: plot_domain::plot::plot_type::PlotType,
) -> Result<DrawableData> {
    match statistic {
        AnyStatistic::Habit(_) => {
            let boolean = value
                .as_bool()
                .ok_or_else(|| anyhow!("Expected boolean for Habit statistic, got: {}", value))?;
            // Habits are rendered as 0 or 1
            Ok(DrawableData::Integer(if boolean { 1 } else { 0 }))
        }
        AnyStatistic::IntegerAmount(_) => {
            let number = value.as_i64().ok_or_else(|| {
                anyhow!(
                    "Expected integer for IntegerAmount statistic, got: {}",
                    value
                )
            })?;
            Ok(DrawableData::Integer(number))
        }
        AnyStatistic::PointInTime(_) => {
            // PointInTime is stored as HH:MM:SS string, but we render it as seconds from midnight
            let time_str = value.as_str().ok_or_else(|| {
                anyhow!("Expected string for PointInTime statistic, got: {}", value)
            })?;
            let naive_time = chrono::NaiveTime::parse_from_str(time_str, "%H:%M:%S")
                .map_err(|e| anyhow!("Failed to parse time '{}': {}", time_str, e))?;
            let seconds_from_midnight = naive_time.num_seconds_from_midnight() as i64;
            Ok(DrawableData::Integer(seconds_from_midnight))
        }
        AnyStatistic::TimeSpans(_) => {
            let time_frame_list = serde_json::from_value::<TimeFrameList>(value.clone())
                .map_err(|e| anyhow!("Failed to parse TimeFrameList: {}", e))?;

            // For bar/line/scatter plots, convert to total duration in seconds
            // For time_frames plot, keep as TimeFrameList for range visualization
            match plot_type {
                plot_domain::plot::plot_type::PlotType::BarChart
                | plot_domain::plot::plot_type::PlotType::LineChart
                | plot_domain::plot::plot_type::PlotType::Scatter => {
                    let total_seconds = time_frame_list.get_total_duration().num_seconds();
                    Ok(DrawableData::Integer(total_seconds))
                }
                plot_domain::plot::plot_type::PlotType::TimeFrames
                | plot_domain::plot::plot_type::PlotType::CircularProgress => {
                    Ok(DrawableData::TimeFrames(time_frame_list))
                }
            }
        }
        AnyStatistic::Summing(_) | AnyStatistic::TimeDifference(_) => {
            // Derived statistics - currently not fully supported for plotting
            // For now, treat them as seconds (integer)
            let seconds = value.as_i64().ok_or_else(|| {
                anyhow!(
                    "Expected integer seconds for derived statistic, got: {}",
                    value
                )
            })?;
            Ok(DrawableData::Integer(seconds))
        }
    }
}

/// Converts a JSON goal value to DrawableGoal based on the statistic type and plot type
///
/// # Arguments
/// * `statistic` - The statistic metadata to determine the expected type
/// * `value` - The JSON goal value from the database
/// * `plot_type` - The type of plot being rendered (affects conversion for TimeSpans)
///
/// # Returns
/// * `Ok(DrawableGoal)` - The typed drawable goal
/// * `Err` - If the JSON structure doesn't match the expected statistic type
pub fn json_value_to_drawable_goal(
    statistic: &AnyStatistic,
    value: &Value,
    plot_type: plot_domain::plot::plot_type::PlotType,
) -> Result<DrawableGoal> {
    match statistic {
        AnyStatistic::Habit(_) => {
            // Habit goals are boolean (true = should be done)
            // Extract the "target" field from the HabitGoal JSON structure
            let goal_value = value
                .get("target")
                .and_then(|v| v.as_bool())
                .ok_or_else(|| {
                    anyhow!(
                        "Expected boolean 'target' field in HabitGoal, got: {}",
                        value
                    )
                })?;
            Ok(DrawableGoal::Integer(if goal_value { 1 } else { 0 }))
        }
        AnyStatistic::IntegerAmount(_) => {
            // IntegerAmount goals have a "target_amount" field with the target number
            let goal_value = value
                .get("target_amount")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| {
                    anyhow!(
                        "Expected integer 'target_amount' field in AmountGoal, got: {}",
                        value
                    )
                })?;
            Ok(DrawableGoal::Integer(goal_value))
        }
        AnyStatistic::PointInTime(_) => {
            // PointInTime goals have a "target" field with the target time
            let time_str = value
                .get("target")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    anyhow!(
                        "Expected string 'target' field in FinishedInTimeGoal, got: {}",
                        value
                    )
                })?;
            let naive_time = chrono::NaiveTime::parse_from_str(time_str, "%H:%M:%S")
                .map_err(|e| anyhow!("Failed to parse goal time '{}': {}", time_str, e))?;
            let seconds_from_midnight = naive_time.num_seconds_from_midnight() as i64;
            Ok(DrawableGoal::Integer(seconds_from_midnight))
        }
        AnyStatistic::TimeSpans(_) => {
            // TimeSpans goals have a "target" field with the target duration as TimeFrameList
            let time_frame_list = value
                .get("target")
                .ok_or_else(|| anyhow!("Expected 'target' field in TimeSpansGoal, got: {}", value))
                .and_then(|v| {
                    serde_json::from_value::<TimeFrameList>(v.clone())
                        .map_err(|e| anyhow!("Failed to parse TimeFrameList from goal: {}", e))
                })?;

            // For bar/line/scatter plots, convert to total duration in seconds
            // For time_frames plot, keep as TimeFrameList for range visualization
            match plot_type {
                plot_domain::plot::plot_type::PlotType::BarChart
                | plot_domain::plot::plot_type::PlotType::LineChart
                | plot_domain::plot::plot_type::PlotType::Scatter => {
                    let total_seconds = time_frame_list.get_total_duration().num_seconds();
                    Ok(DrawableGoal::Integer(total_seconds))
                }
                plot_domain::plot::plot_type::PlotType::TimeFrames
                | plot_domain::plot::plot_type::PlotType::CircularProgress => {
                    Ok(DrawableGoal::TimeFrames(time_frame_list))
                }
            }
        }
        AnyStatistic::Summing(_) | AnyStatistic::TimeDifference(_) => {
            // Derived statistics - goals would be in seconds
            let seconds = value.get("value").and_then(|v| v.as_i64()).ok_or_else(|| {
                anyhow!(
                    "Expected integer 'value' field for derived statistic goal, got: {}",
                    value
                )
            })?;
            Ok(DrawableGoal::Integer(seconds))
        }
    }
}

/// Maps goal achievement percentage to an RGB color
///
/// This function encodes the business logic of how goal achievement is visually represented.
/// The color scheme provides clear visual feedback on performance:
///
/// # Color Scheme
/// * **Gold** `[255, 215, 0]` - >= 150%: Exceeded goal significantly
/// * **Green** `[169, 252, 177]` - >= 100%: Goal achieved
/// * **Orange** `[255, 174, 113]` - >= 90%: Close to goal
/// * **Red** `[245, 169, 142]` - < 90%: Below goal
/// * **White** `[255, 255, 255]` - No goal: No target set
///
/// # Arguments
/// * `percentage` - Optional goal achievement percentage
///
/// # Returns
/// RGB color tuple representing the achievement level
pub fn percentage_to_color(percentage: Option<Percentage>) -> [u8; 3] {
    match percentage {
        Some(p) => {
            let value = p.as_num();
            if value >= 150 {
                [255, 215, 0] // Gold - exceeded goal significantly
            } else if value >= 100 {
                [169, 252, 177] // Green - goal achieved
            } else if value >= 90 {
                [255, 174, 113] // Orange - close to goal
            } else {
                [245, 169, 142] // Red - below goal
            }
        }
        None => [255, 255, 255], // White - no goal set
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::types::logable_statistics::habit::HabitStatistic;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use serde_json::json;

    #[test]
    fn test_habit_to_drawable_data() {
        let stat = AnyStatistic::Habit(HabitStatistic::new(
            StatisticMetadata::new(StatisticId::from(1), "Test".into(), "Test Habit".into()),
            None,
            chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        ));

        let result_true = json_value_to_drawable_data(
            &stat,
            &json!(true),
            plot_domain::plot::plot_type::PlotType::BarChart,
        )
        .unwrap();
        assert_eq!(result_true, DrawableData::Integer(1));

        let result_false = json_value_to_drawable_data(
            &stat,
            &json!(false),
            plot_domain::plot::plot_type::PlotType::BarChart,
        )
        .unwrap();
        assert_eq!(result_false, DrawableData::Integer(0));
    }

    #[test]
    fn test_integer_amount_to_drawable_data() {
        let stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(StatisticId::from(1), "Test".into(), "Test Amount".into()),
            None,
            chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        ));

        let result = json_value_to_drawable_data(
            &stat,
            &json!(42),
            plot_domain::plot::plot_type::PlotType::BarChart,
        )
        .unwrap();
        assert_eq!(result, DrawableData::Integer(42));
    }

    #[test]
    fn test_time_spans_to_drawable_data_for_time_frames_plot() {
        let stat = AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            StatisticMetadata::new(StatisticId::from(1), "Test".into(), "Test Spans".into()),
            None,
            chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        ));

        let time_frames_json = json!([
            {"start": "09:00:00", "end": "12:00:00"},
            {"start": "14:00:00", "end": "18:00:00"}
        ]);

        // TimeFrames plot type should return DrawableData::TimeFrames
        let result = json_value_to_drawable_data(
            &stat,
            &time_frames_json,
            plot_domain::plot::plot_type::PlotType::TimeFrames,
        )
        .unwrap();
        match result {
            DrawableData::TimeFrames(tf) => {
                // Verify we got a TimeFrameList
                assert_eq!(tf.iter().count(), 2);
            }
            _ => panic!("Expected TimeFrames"),
        }
    }

    #[test]
    fn test_time_spans_to_drawable_data_for_bar_chart() {
        let stat = AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            StatisticMetadata::new(StatisticId::from(1), "Test".into(), "Test Spans".into()),
            None,
            chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        ));

        let time_frames_json = json!([
            {"start": "09:00:00", "end": "12:00:00"},  // 3 hours = 10800 seconds
            {"start": "14:00:00", "end": "18:00:00"}   // 4 hours = 14400 seconds
        ]);
        // Total: 7 hours = 25200 seconds

        // Bar chart plot type should return DrawableData::Integer (total duration in seconds)
        let result = json_value_to_drawable_data(
            &stat,
            &time_frames_json,
            plot_domain::plot::plot_type::PlotType::BarChart,
        )
        .unwrap();
        match result {
            DrawableData::Integer(seconds) => {
                // Verify we got total duration in seconds (7 hours = 25200 seconds)
                assert_eq!(seconds, 25200);
            }
            _ => panic!("Expected Integer"),
        }
    }

    #[test]
    fn test_integer_amount_goal_to_drawable() {
        let stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(StatisticId::from(1), "Test".into(), "Test".into()),
            None,
            chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        ));

        // AmountGoal JSON structure has a "target_amount" field
        let goal_json = json!({"target_amount": 100, "is_at_least": true});
        let result = json_value_to_drawable_goal(
            &stat,
            &goal_json,
            plot_domain::plot::plot_type::PlotType::BarChart,
        )
        .unwrap();
        assert_eq!(result, DrawableGoal::Integer(100));
    }

    #[test]
    fn test_habit_goal_to_drawable() {
        let stat = AnyStatistic::Habit(HabitStatistic::new(
            StatisticMetadata::new(StatisticId::from(1), "Test".into(), "Test".into()),
            None,
            chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        ));

        // HabitGoal JSON structure has a "target" field with boolean
        let goal_json = json!({"target": true});
        let result = json_value_to_drawable_goal(
            &stat,
            &goal_json,
            plot_domain::plot::plot_type::PlotType::BarChart,
        )
        .unwrap();
        assert_eq!(result, DrawableGoal::Integer(1));
    }

    #[test]
    fn test_percentage_to_color_no_goal() {
        assert_eq!(percentage_to_color(None), [255, 255, 255]); // White
    }

    #[test]
    fn test_percentage_to_color_gold() {
        assert_eq!(
            percentage_to_color(Some(Percentage::new(150))),
            [255, 215, 0]
        ); // Gold
        assert_eq!(
            percentage_to_color(Some(Percentage::new(200))),
            [255, 215, 0]
        ); // Gold
    }

    #[test]
    fn test_percentage_to_color_achieved() {
        assert_eq!(
            percentage_to_color(Some(Percentage::new(100))),
            [169, 252, 177]
        ); // Green
        assert_eq!(
            percentage_to_color(Some(Percentage::new(120))),
            [169, 252, 177]
        ); // Green
        assert_eq!(
            percentage_to_color(Some(Percentage::new(149))),
            [169, 252, 177]
        ); // Green
    }

    #[test]
    fn test_percentage_to_color_close() {
        assert_eq!(
            percentage_to_color(Some(Percentage::new(90))),
            [255, 174, 113]
        ); // Orange
        assert_eq!(
            percentage_to_color(Some(Percentage::new(95))),
            [255, 174, 113]
        ); // Orange
        assert_eq!(
            percentage_to_color(Some(Percentage::new(99))),
            [255, 174, 113]
        ); // Orange
    }

    #[test]
    fn test_percentage_to_color_below() {
        assert_eq!(
            percentage_to_color(Some(Percentage::new(0))),
            [245, 169, 142]
        ); // Red
        assert_eq!(
            percentage_to_color(Some(Percentage::new(50))),
            [245, 169, 142]
        ); // Red
        assert_eq!(
            percentage_to_color(Some(Percentage::new(89))),
            [245, 169, 142]
        ); // Red
    }
}
