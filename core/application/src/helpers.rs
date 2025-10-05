use anyhow::{anyhow, Result};
use lifestats_domain::plot::data::DrawableData;
use lifestats_domain::plot::specification::PlotType;
use lifestats_domain::shared::percentage::Percentage;
use lifestats_domain::shared::time_frame_list::TimeFrameList;
use lifestats_domain::statistic::any_statistic::AnyStatistic;
use lifestats_domain::statistic::value_objects::goals::amount_goal::AmountGoal;
use lifestats_domain::statistic::value_objects::goals::finished_in_time_goal::FinishedInTimeGoal;
use lifestats_domain::statistic::value_objects::goals::goal_trait::Goal;
use lifestats_domain::statistic::value_objects::goals::habit_goal::HabitGoal;
use lifestats_domain::statistic::value_objects::goals::time_spans_goal::TimeSpansGoal;
use serde_json::Value;

/// Maps goal achievement percentage to an RGB color tuple
///
/// Color scheme:
/// - >= 100%: Green [0, 255, 0]
/// - 90-99%: Yellow [255, 255, 0]
/// - < 90%: Red [255, 0, 0]
/// - No goal: Gray [128, 128, 128]
pub fn percentage_to_color(percentage: Option<Percentage>) -> [u8; 3] {
    match percentage {
        Some(p) => {
            let value = p.as_num();
            if value >= 100 {
                [0, 255, 0] // Green - goal achieved or exceeded
            } else if value >= 90 {
                [255, 255, 0] // Yellow - close to goal
            } else {
                [255, 0, 0] // Red - below goal
            }
        }
        None => [128, 128, 128], // Gray - no goal set
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

/// Evaluates goal achievement for a given statistic and returns a percentage
/// Returns None if no goal is provided
pub fn evaluate_goal_achievement(
    statistic: &AnyStatistic,
    log_value: &Value,
    goal_value: Option<&Value>,
) -> Result<Option<Percentage>> {
    let goal = match goal_value {
        Some(g) => g,
        None => return Ok(None),
    };

    match statistic {
        AnyStatistic::Habit(_) => {
            let value = log_value
                .as_bool()
                .ok_or_else(|| anyhow!("Expected bool for Habit statistic"))?;
            let goal_obj = serde_json::from_value::<HabitGoal>(goal.clone())
                .map_err(|e| anyhow!("Failed to parse HabitGoal: {}", e))?;
            Ok(Some(goal_obj.get_percentage_achieved(Some(&value))))
        }
        AnyStatistic::IntegerAmount(_) => {
            let value = log_value
                .as_i64()
                .ok_or_else(|| anyhow!("Expected i64 for IntegerAmount statistic"))?;
            let goal_obj = serde_json::from_value::<AmountGoal<i64>>(goal.clone())
                .map_err(|e| anyhow!("Failed to parse AmountGoal<i64>: {}", e))?;
            Ok(Some(goal_obj.get_percentage_achieved(Some(&value))))
        }
        AnyStatistic::PointInTime(_) => {
            let value = serde_json::from_value::<chrono::NaiveTime>(log_value.clone())
                .map_err(|e| anyhow!("Failed to parse NaiveTime: {}", e))?;
            let goal_obj = serde_json::from_value::<FinishedInTimeGoal>(goal.clone())
                .map_err(|e| anyhow!("Failed to parse FinishedInTimeGoal: {}", e))?;
            Ok(Some(goal_obj.get_percentage_achieved(Some(&value))))
        }
        AnyStatistic::TimeSpans(_) => {
            let value = serde_json::from_value::<TimeFrameList>(log_value.clone())
                .map_err(|e| anyhow!("Failed to parse TimeFrameList: {}", e))?;
            let goal_obj = serde_json::from_value::<TimeSpansGoal>(goal.clone())
                .map_err(|e| anyhow!("Failed to parse TimeSpansGoal: {}", e))?;
            Ok(Some(goal_obj.get_percentage_achieved(Some(&value))))
        }
        AnyStatistic::Summing(_) | AnyStatistic::TimeDifference(_) => {
            // These use Seconds, which we'll skip for now as they're derived statistics
            // and typically not directly plotted with goals
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentage_to_color_no_goal() {
        assert_eq!(percentage_to_color(None), [128, 128, 128]);
    }

    #[test]
    fn test_percentage_to_color_achieved() {
        assert_eq!(percentage_to_color(Some(Percentage::new(100))), [0, 255, 0]);
        assert_eq!(percentage_to_color(Some(Percentage::new(150))), [0, 255, 0]);
    }

    #[test]
    fn test_percentage_to_color_close() {
        assert_eq!(
            percentage_to_color(Some(Percentage::new(90))),
            [255, 255, 0]
        );
        assert_eq!(
            percentage_to_color(Some(Percentage::new(95))),
            [255, 255, 0]
        );
    }

    #[test]
    fn test_percentage_to_color_below() {
        assert_eq!(percentage_to_color(Some(Percentage::new(0))), [255, 0, 0]);
        assert_eq!(percentage_to_color(Some(Percentage::new(50))), [255, 0, 0]);
        assert_eq!(percentage_to_color(Some(Percentage::new(89))), [255, 0, 0]);
    }
}
