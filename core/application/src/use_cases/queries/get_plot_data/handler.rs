use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use crate::services::goal_resolver::GoalResolver;
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
    /// Accepts optional log_value to support goal-only days.
    fn evaluate_goal_achievement(
        &self,
        statistic: &AnyStatistic,
        log_value: Option<&serde_json::Value>,
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
                let parsed_value = log_value.and_then(|v| v.as_bool());
                let goal_obj = serde_json::from_value::<HabitGoal>(goal.clone())
                    .map_err(|e| anyhow!("Failed to parse HabitGoal: {}", e))?;
                Ok(Some(
                    goal_obj.get_percentage_achieved(parsed_value.as_ref()),
                ))
            }
            AnyStatistic::IntegerAmount(_) => {
                let parsed_value = log_value.and_then(|v| v.as_i64());
                let goal_obj = serde_json::from_value::<AmountGoal<i64>>(goal.clone())
                    .map_err(|e| anyhow!("Failed to parse AmountGoal<i64>: {}", e))?;
                Ok(Some(
                    goal_obj.get_percentage_achieved(parsed_value.as_ref()),
                ))
            }
            AnyStatistic::PointInTime(_) => {
                let parsed_value = log_value
                    .map(|v| serde_json::from_value::<chrono::NaiveTime>(v.clone()))
                    .transpose()
                    .map_err(|e| anyhow!("Failed to parse NaiveTime: {}", e))?;
                let goal_obj = serde_json::from_value::<FinishedInTimeGoal>(goal.clone())
                    .map_err(|e| anyhow!("Failed to parse FinishedInTimeGoal: {}", e))?;
                Ok(Some(
                    goal_obj.get_percentage_achieved(parsed_value.as_ref()),
                ))
            }
            AnyStatistic::TimeSpans(_) => {
                let parsed_value = log_value
                    .map(|v| {
                        serde_json::from_value::<
                            lifestats_core::shared::time_frame_list::TimeFrameList,
                        >(v.clone())
                    })
                    .transpose()
                    .map_err(|e| anyhow!("Failed to parse TimeFrameList: {}", e))?;
                let goal_obj = serde_json::from_value::<TimeSpansGoal>(goal.clone())
                    .map_err(|e| anyhow!("Failed to parse TimeSpansGoal: {}", e))?;
                Ok(Some(
                    goal_obj.get_percentage_achieved(parsed_value.as_ref()),
                ))
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

        // 2. Load log values for the date range
        let log_values = self
            .statistic_repository
            .find_log_values_in_range(query.statistic_id, query.date_range)?;

        // 3. Resolve goals for the range using GoalResolver
        let goal_resolver = GoalResolver::new(self.statistic_repository.clone());
        let goals = goal_resolver.resolve_goals_for_range(&statistic, query.date_range)?;

        // 4. Build a map of dates to log values for efficient lookup
        let log_map: std::collections::HashMap<_, _> = log_values
            .into_iter()
            .map(|log| (log.date, log.value))
            .collect();

        // 5. Iterate over all dates in the range, creating DataPoints for days with logs or goals
        let mut data_points = Vec::new();

        let mut current = query.date_range.start();
        while current <= query.date_range.end() {
            // Look up log value for this date
            let log_value_json = log_map.get(&current);

            // Get resolved goal for this date
            let goal_for_day = goals.get(&current).and_then(|opt| opt.as_ref());

            // Skip days with neither log nor goal
            if log_value_json.is_none() && goal_for_day.is_none() {
                current = current.succ_opt().ok_or_else(|| anyhow!("Date overflow"))?;
                continue;
            }

            // Compute percentage achieved (business logic)
            let percentage =
                self.evaluate_goal_achievement(&statistic, log_value_json, goal_for_day)?;

            // Calculate color based on goal achievement
            let color = percentage_to_color(percentage);

            // Convert JSON value to DrawableData (if present)
            let drawable_value = log_value_json
                .map(|v| json_value_to_drawable_data(&statistic, v))
                .transpose()?;

            // Convert JSON goal to DrawableGoal (if present)
            let drawable_goal = goal_for_day
                .map(|g| json_value_to_drawable_goal(&statistic, g))
                .transpose()?;

            // Create the data point with all typed, renderer-ready data
            data_points.push(DataPoint::new(
                current,
                drawable_value,
                color,
                drawable_goal,
            ));

            current = current.succ_opt().ok_or_else(|| anyhow!("Date overflow"))?;
        }

        // 6. Calculate Y-axis from data points
        let y_axis = YAxis::from_data_points(&data_points, query.plot_type)?;

        // 7. Build and return complete PlotData
        let title = statistic.get_metadata().name().to_string();
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
    use lifestats_core::statistic::types::logable_statistics::habit::HabitStatistic;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::value_objects::goals::amount_goal::AmountGoal;
    use lifestats_core::statistic::value_objects::goals::goal_query_result::GoalQueryResult;
    use lifestats_core::statistic::value_objects::goals::habit_goal::HabitGoal;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use plot_domain::plot::data::DrawableData;
    use plot_domain::plot::plot_type::PlotType;
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

        // Verify we have data points for both days (day with log + day with goal only)
        assert_eq!(plot_data.points.len(), 2);

        // Verify first point (2025-1-1) has both value and goal
        let point1 = &plot_data.points[0];
        assert_eq!(point1.date, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
        match &point1.value {
            Some(DrawableData::Integer(val)) => {
                assert_eq!(*val, 15);
            }
            _ => panic!("Expected Some(Integer DrawableData)"),
        }
        // Verify color is gold (150% achievement)
        assert_eq!(point1.color, [255, 215, 0]); // Gold

        // Verify second point (2025-1-2) has goal but no value
        let point2 = &plot_data.points[1];
        assert_eq!(point2.date, NaiveDate::from_ymd_opt(2025, 1, 2).unwrap());
        assert!(
            point2.value.is_none(),
            "Second day should have no logged value"
        );
        assert!(point2.goal.is_some(), "Second day should have a goal");

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

    #[test]
    fn test_habit_plot_with_goal_but_no_logged_value() {
        // This test is a regression test for a bug where plotting a Habit statistic
        // would error with "Expected bool for Habit statistic" when a date had a goal
        // but no logged value. This is a valid scenario (e.g., future dates with goals).

        // Arrange
        let mut stat_repo = MockStatisticRepository::new();

        let stat_id = StatisticId::from(60);
        let date_with_value = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let date_without_value = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
        let date_range = DateRange::new(date_with_value, date_without_value).unwrap();

        // Create a Habit statistic with default goal (target is to occur = true)
        let default_goal = HabitGoal::new(true);
        let habit_stat =
            lifestats_core::statistic::any_statistic::AnyStatistic::Habit(HabitStatistic::new(
                StatisticMetadata::new(stat_id, "Health".into(), "Daily Exercise".into()),
                Some(default_goal),
                date_with_value,
            ));

        stat_repo
            .expect_find_metadata_by_id()
            .returning(move |_| Ok(habit_stat.clone()));

        // Only ONE day has a logged value (true = habit occurred)
        stat_repo
            .expect_find_log_values_in_range()
            .returning(|_, _| {
                Ok(vec![DatedValue::new(
                    NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    json!(true), // Did the habit on day 1
                )])
            });

        // No daily goal overrides - using default goal for both dates
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

        // Assert - should NOT error even though day 2 has no logged value
        assert!(
            result.is_ok(),
            "Should not error when Habit has goal but no logged value"
        );
        let plot_data = result.unwrap();

        // Verify we have data points for both dates
        assert_eq!(
            plot_data.points.len(),
            2,
            "Should have points for both dates (one with value, one goal-only)"
        );

        // Day 1: Has value=true, goal=true -> 100% achievement (target met)
        let point1 = &plot_data.points[0];
        assert_eq!(point1.date, date_with_value);
        match &point1.value {
            Some(DrawableData::Integer(val)) => {
                assert_eq!(*val, 1, "Habit occurrence should be represented as 1");
            }
            _ => panic!("Expected Some(Integer DrawableData) for logged habit"),
        }
        assert!(point1.goal.is_some(), "Day 1 should have a goal");
        assert_eq!(
            point1.color,
            [169, 252, 177],
            "Should be green (100% - goal met)"
        );

        // Day 2: No logged value, goal=true -> defaults to false -> 0% achievement
        let point2 = &plot_data.points[1];
        assert_eq!(point2.date, date_without_value);
        assert_eq!(
            point2.value, None,
            "Day 2 should have no logged value (this was the bug)"
        );
        assert!(point2.goal.is_some(), "Day 2 should still have a goal");
        assert_eq!(
            point2.color,
            [245, 169, 142],
            "Should be red (0% - no value defaults to false, goal not met)"
        );
    }
}
