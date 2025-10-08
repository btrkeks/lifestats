use anyhow::{anyhow, Result};
use lifestats_core::shared::percentage::Percentage;
use lifestats_core::shared::time_frame_list::TimeFrameList;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::value_objects::goals::amount_goal::AmountGoal;
use lifestats_core::statistic::value_objects::goals::habit_goal::HabitGoal;
use lifestats_core::statistic::value_objects::goals::time_spans_goal::TimeSpansGoal;
use plot_domain::plot::data::{DrawableData, DrawableGoal};
use plot_domain::plot::specification::PlotType;
use serde_json::Value;

/// Maps goal achievement percentage to an RGB color tuple
///
/// Color scheme (matching legacy colors):
/// - >= 150%: Gold [255, 215, 0]
/// - >= 100%: Green [169, 252, 177]
/// - >= 90%: Orange [255, 174, 113]
/// - < 90%: Red [245, 169, 142]
/// - No goal: White [255, 255, 255]
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

/// Converts a JSON value to DrawableData based on the statistic's capabilities and plot type.
pub fn json_to_drawable_data(
    statistic: &AnyStatistic,
    value: &Value,
    plot_type: PlotType,
) -> Result<DrawableData> {
    match plot_type {
        PlotType::TimeFrames => {
            let time_frame_list = serde_json::from_value::<TimeFrameList>(value.clone())
                .map_err(|e| anyhow!("Expected TimeFrameList for TimeFrames plot: {}", e))?;
            Ok(DrawableData::TimeFrames(time_frame_list))
        }
        PlotType::BarChart
        | PlotType::LineChart
        | PlotType::Scatter
        | PlotType::CircularProgress => {
            let int_value = statistic.value_as_i64_from_json(value).map_err(|_| {
                anyhow!(
                    "Statistic '{}' cannot be represented as an integer for a {:?} plot.",
                    statistic.get_metadata().name(),
                    plot_type
                )
            })?;
            Ok(DrawableData::Integer(int_value))
        }
    }
}

/// Converts a JSON goal value to DrawableGoal based on the statistic type
/// This function unwraps complex goal objects (like TimeSpansGoal) into simple drawable types
pub fn json_to_drawable_goal(statistic: &AnyStatistic, goal: &Value) -> Result<DrawableGoal> {
    match statistic {
        AnyStatistic::TimeSpans(_) => {
            // Parse the full TimeSpansGoal object, then extract the target TimeFrameList
            let time_spans_goal = serde_json::from_value::<TimeSpansGoal>(goal.clone())
                .map_err(|e| anyhow!("Failed to parse TimeSpansGoal: {}", e))?;
            Ok(DrawableGoal::TimeFrames(time_spans_goal.target().clone()))
        }
        AnyStatistic::Habit(_) => {
            let habit_goal = serde_json::from_value::<HabitGoal>(goal.clone())
                .map_err(|e| anyhow!("Failed to parse HabitGoal: {}", e))?;
            Ok(DrawableGoal::Integer(habit_goal.occurred() as i64))
        }
        AnyStatistic::IntegerAmount(_) => {
            let amount_goal = serde_json::from_value::<AmountGoal<i64>>(goal.clone())
                .map_err(|e| anyhow!("Failed to parse AmountGoal: {}", e))?;
            Ok(DrawableGoal::Integer(amount_goal.target_amount()))
        }
        AnyStatistic::PointInTime(_) => {
            // PointInTime goals are not typically displayed on plots, but we can convert them
            // For now, return an error as these are not supported
            Err(anyhow!(
                "PointInTime goals are not yet supported for drawable goals"
            ))
        }
        AnyStatistic::Summing(_) | AnyStatistic::TimeDifference(_) => {
            // Derived statistics typically don't have goals
            Err(anyhow!("Derived statistics do not support goals"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        ); // Green (just below gold)
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
