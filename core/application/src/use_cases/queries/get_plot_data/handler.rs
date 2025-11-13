use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use crate::services::plot_data_builder::{
    json_value_to_drawable_data, json_value_to_drawable_goal, percentage_to_color,
};
use anyhow::anyhow;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::value_objects::goals::goal_trait::Goal;
use plot_domain::plot::axis::YAxis;
use plot_domain::plot::data::{DataPoint, PlotData};

use super::query::GetPlotDataQuery;

/// Handler for GetPlotDataQuery (Stateless Plotting)
///
/// This handler builds a complete PlotData domain object on-the-fly without
/// requiring any stored plot configuration. It:
/// - Fetches log values and goals from the repository
/// - Computes goal achievement percentages (business logic)
/// - Converts JSON values to DrawableData/DrawableGoal (using application services)
/// - Calculates colors based on goal achievement (business logic)
/// - Computes the Y-axis configuration (domain logic)
/// - Returns a fully-populated PlotData ready for rendering
///
/// This replaces the old stateful approach that required PlotRepository.
pub struct GetPlotDataQueryHandler<S>
where
    S: StatisticRepository + Send + Sync + 'static,
{
    statistic_repository: Arc<S>,
}

impl<S> GetPlotDataQueryHandler<S>
where
    S: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(statistic_repository: Arc<S>) -> Self {
        Self {
            statistic_repository,
        }
    }

    /// Evaluate goal achievement for a log value
    ///
    /// This is business logic - determining if a goal was achieved.
    /// It returns a percentage, NOT a color.
    fn evaluate_goal_achievement(
        &self,
        statistic: &AnyStatistic,
        log_value: &serde_json::Value,
        goal_value: Option<&serde_json::Value>,
    ) -> ApplicationResult<Option<lifestats_core::shared::percentage::Percentage>> {
        use lifestats_core::statistic::value_objects::goals::{
            amount_goal::AmountGoal, finished_in_time_goal::FinishedInTimeGoal,
            habit_goal::HabitGoal, time_spans_goal::TimeSpansGoal,
        };

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
                let value = serde_json::from_value::<
                    lifestats_core::shared::time_frame_list::TimeFrameList,
                >(log_value.clone())
                .map_err(|e| anyhow!("Failed to parse TimeFrameList: {}", e))?;
                let goal_obj = serde_json::from_value::<TimeSpansGoal>(goal.clone())
                    .map_err(|e| anyhow!("Failed to parse TimeSpansGoal: {}", e))?;
                Ok(Some(goal_obj.get_percentage_achieved(Some(&value))))
            }
            AnyStatistic::Summing(_) | AnyStatistic::TimeDifference(_) => {
                // These use Seconds - skip for now as they're derived statistics
                Ok(None)
            }
        }
    }
}

impl<S> QueryHandler<GetPlotDataQuery> for GetPlotDataQueryHandler<S>
where
    S: StatisticRepository + Send + Sync + 'static,
{
    fn handle(&self, query: GetPlotDataQuery) -> ApplicationResult<PlotData> {
        // 1. Load statistic metadata
        let statistic = self
            .statistic_repository
            .find_metadata_by_id(query.statistic_id)?;

        // 2. Load log values and per-day goals for the date range
        let log_values = self
            .statistic_repository
            .find_log_values_in_range(query.statistic_id, query.date_range)?;

        let daily_goals = self
            .statistic_repository
            .find_goals_in_range(query.statistic_id, query.date_range)?;

        // 3. Create a map of dates to goal values for efficient lookup
        let daily_goal_map: std::collections::HashMap<_, _> = daily_goals
            .into_iter()
            .map(|g| (g.date, g.value.clone()))
            .collect();

        // 4. Get the default goal and the date it's valid from
        let default_goal_valid_since = statistic.get_default_goal_valid_since();
        let default_goal_json = statistic.get_default_goal().and_then(|any_goal| {
            use lifestats_core::statistic::any_statistic::AnyGoal;
            match any_goal {
                AnyGoal::Habit(g) => serde_json::to_value(g).ok(),
                AnyGoal::IntegerAmount(g) => serde_json::to_value(g).ok(),
                AnyGoal::FinishedInTime(g) => serde_json::to_value(g).ok(),
                AnyGoal::TimeSpans(g) => serde_json::to_value(g).ok(),
                AnyGoal::SecondsAmount(g) => serde_json::to_value(g).ok(),
            }
        });

        // 5. Convert log values to DataPoints with DrawableData and colors
        let data_points: ApplicationResult<Vec<DataPoint>> = log_values
            .into_iter()
            .map(|log| {
                // Determine the correct goal for this date
                let goal_for_day = daily_goal_map.get(&log.date).or_else(|| {
                    if log.date >= default_goal_valid_since {
                        default_goal_json.as_ref()
                    } else {
                        None
                    }
                });

                // Compute percentage achieved (business logic)
                let percentage =
                    self.evaluate_goal_achievement(&statistic, &log.value, goal_for_day)?;

                // Calculate color based on goal achievement
                let color = percentage_to_color(percentage);

                // Convert JSON value to DrawableData
                let drawable_value = json_value_to_drawable_data(&statistic, &log.value)?;

                // Convert JSON goal to DrawableGoal (if present)
                let drawable_goal = goal_for_day
                    .map(|g| json_value_to_drawable_goal(&statistic, g))
                    .transpose()?;

                // Create the data point with all typed, renderer-ready data
                Ok(DataPoint::new(
                    log.date,
                    drawable_value,
                    color,
                    drawable_goal,
                ))
            })
            .collect();

        let data_points = data_points?;

        // 6. Calculate Y-axis from data points
        let y_axis = YAxis::from_data_points(&data_points, query.plot_type)?;

        // 7. Build and return complete PlotData
        let title = format!("{}", statistic.get_metadata().name());
        Ok(PlotData::new(
            title,
            data_points,
            query.date_range,
            query.plot_type,
            y_axis,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_core::shared::date_range::DateRange;
    use lifestats_core::shared::dated_value::DatedValue;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::value_objects::goals::amount_goal::AmountGoal;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use plot_domain::plot::data::DrawableData;
    use plot_domain::plot::specification::PlotType;
    use serde_json::json;

    #[test]
    fn test_get_plot_data_returns_complete_plot_data() {
        // Arrange
        let mut stat_repo = MockStatisticRepository::new();

        let stat_id = StatisticId::from(42);
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
        let date_range = DateRange::new(start, end).unwrap();

        // Create a test statistic with default goal
        let default_goal = AmountGoal::new(10, true);
        let test_stat = lifestats_core::statistic::any_statistic::AnyStatistic::IntegerAmount(
            IntegerAmountStatistic::new(
                StatisticMetadata::new(stat_id, "Test Category".into(), "Test Stat".into()),
                Some(default_goal),
                start,
            ),
        );

        stat_repo
            .expect_find_metadata_by_id()
            .returning(move |_| Ok(test_stat.clone()));

        // Mock log values
        stat_repo
            .expect_find_log_values_in_range()
            .returning(|_, _| {
                Ok(vec![DatedValue::new(
                    NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    json!(15), // Achieves goal of 10 (150%)
                )])
            });

        stat_repo
            .expect_find_goals_in_range()
            .returning(|_, _| Ok(vec![]));

        let handler = GetPlotDataQueryHandler::new(Arc::new(stat_repo));
        let query = GetPlotDataQuery {
            statistic_id: stat_id,
            plot_type: PlotType::LineChart,
            date_range,
        };

        // Act
        let result = handler.handle(query);

        // Assert
        assert!(result.is_ok());
        let plot_data = result.unwrap();

        // Verify title is the statistic name
        assert_eq!(plot_data.title, "Test Stat");

        // Verify we have data points
        assert_eq!(plot_data.points.len(), 1);

        // Verify the point contains DrawableData (not raw JSON)
        let point = &plot_data.points[0];
        match &point.value {
            DrawableData::Integer(val) => {
                assert_eq!(*val, 15);
            }
            _ => panic!("Expected Integer DrawableData"),
        }

        // Verify color is gold (150% achievement)
        assert_eq!(point.color, [255, 215, 0]); // Gold

        // Verify plot type
        assert_eq!(plot_data.plot_type, PlotType::LineChart);

        // Verify Y-axis was calculated
        // With value 15 and goal 10, axis should include both
        match plot_data.y_axis {
            YAxis::Integer { min, max } => {
                assert!(min <= 10, "Min should be <= 10");
                assert!(max >= 15, "Max should be >= 15");
            }
            _ => panic!("Expected Integer YAxis"),
        }
    }

    #[test]
    fn test_get_plot_data_with_no_data() {
        // Arrange
        let mut stat_repo = MockStatisticRepository::new();

        let stat_id = StatisticId::from(42);
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
        let date_range = DateRange::new(start, end).unwrap();

        // Create a test statistic without data
        let test_stat = lifestats_core::statistic::any_statistic::AnyStatistic::IntegerAmount(
            IntegerAmountStatistic::new(
                StatisticMetadata::new(stat_id, "Test Category".into(), "Empty Stat".into()),
                None,
                start,
            ),
        );

        stat_repo
            .expect_find_metadata_by_id()
            .returning(move |_| Ok(test_stat.clone()));

        // No log values
        stat_repo
            .expect_find_log_values_in_range()
            .returning(|_, _| Ok(vec![]));

        stat_repo
            .expect_find_goals_in_range()
            .returning(|_, _| Ok(vec![]));

        let handler = GetPlotDataQueryHandler::new(Arc::new(stat_repo));
        let query = GetPlotDataQuery {
            statistic_id: stat_id,
            plot_type: PlotType::BarChart,
            date_range,
        };

        // Act
        let result = handler.handle(query);

        // Assert - should succeed with empty data points
        assert!(result.is_ok());
        let plot_data = result.unwrap();
        assert_eq!(plot_data.points.len(), 0);

        // Y-axis should have default range for empty data
        assert_eq!(plot_data.y_axis, YAxis::Integer { min: 0, max: 100 });
    }
}
