use std::sync::Arc;

use anyhow::{anyhow, Result};
use lifestats_domain::plot::axis::YAxis;
use lifestats_domain::plot::data::{DataPoint, PlotData};
use lifestats_domain::plot::renderer::{PlotRenderer, RenderConfig};
use lifestats_domain::plot::repository::PlotRepository;
use lifestats_domain::shared::date_range::DateRange;
use lifestats_domain::shared::ids::PlotId;
use lifestats_domain::statistic::ports::statistic_repository::StatisticRepository;

use crate::helpers::{
    evaluate_goal_achievement, json_to_drawable_data, json_to_drawable_goal, percentage_to_color,
};

/// Application service for plotting operations
/// Orchestrates domain objects and repositories to fulfill plotting use cases
#[derive(Clone)]
pub struct PlottingService<S, P, R>
where
    S: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    R: PlotRenderer + 'static,
{
    statistic_repository: Arc<S>,
    plot_repository: Arc<P>,
    renderer: Arc<R>,
}

impl<S, P, R> PlottingService<S, P, R>
where
    S: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
    R: PlotRenderer + 'static,
{
    pub fn new(statistic_repository: Arc<S>, plot_repository: Arc<P>, renderer: Arc<R>) -> Self {
        Self {
            statistic_repository,
            plot_repository,
            renderer,
        }
    }

    /// Retrieves and assembles plot-ready data for a given plot ID and date range
    /// This is a reusable, testable unit for fetching plot data
    pub fn get_plot_data(&self, plot_id: PlotId, date_range: DateRange) -> Result<PlotData> {
        // 1. Load the plot specification
        let plot_spec = self
            .plot_repository
            .find_by_id(plot_id)?
            .ok_or_else(|| anyhow!("Plot not found: {}", plot_id))?;

        // 2. Extract the statistic ID
        let statistic_id = plot_spec.statistic_id;

        // 3. Load statistic metadata for goal achievement logic
        let statistic = self
            .statistic_repository
            .find_metadata_by_id(statistic_id)?;

        // 4. Load log values and per-day goals for the date range
        let log_values = self
            .statistic_repository
            .find_log_values_in_range(statistic_id, date_range)?;

        let daily_goals = self
            .statistic_repository
            .find_goals_in_range(statistic_id, date_range)?;

        // 5. Create a map of dates to goal values for efficient lookup
        let daily_goal_map: std::collections::HashMap<_, _> = daily_goals
            .into_iter()
            .map(|g| (g.date, g.value.clone()))
            .collect();

        // 6. Get the default goal and the date it's valid from
        let default_goal_valid_since = statistic.get_default_goal_valid_since();
        let default_goal_json = statistic.get_default_goal().and_then(|any_goal| {
            // The renderer expects a JSON Value, so we must serialize the typed goal
            // We match on the AnyGoal and serialize the inner type to avoid enum wrapper
            use lifestats_domain::statistic::any_statistic::AnyGoal;
            match any_goal {
                AnyGoal::Habit(g) => serde_json::to_value(g).ok(),
                AnyGoal::IntegerAmount(g) => serde_json::to_value(g).ok(),
                AnyGoal::FinishedInTime(g) => serde_json::to_value(g).ok(),
                AnyGoal::TimeSpans(g) => serde_json::to_value(g).ok(),
                AnyGoal::SecondsAmount(g) => serde_json::to_value(g).ok(),
            }
        });

        // 7. Convert log values to data points with DrawableData and colors
        let points: Result<Vec<DataPoint>> = log_values
            .into_iter()
            .map(|log| {
                // Convert JSON value to DrawableData (type-safe!)
                let drawable_value =
                    json_to_drawable_data(&statistic, &log.value, plot_spec.plot_type)?;

                // Determine the correct goal for this date: daily override or default
                let goal_for_day = daily_goal_map.get(&log.date).or_else(|| {
                    if log.date >= default_goal_valid_since {
                        default_goal_json.as_ref()
                    } else {
                        None
                    }
                });

                // Evaluate goal achievement and calculate color
                let percentage = evaluate_goal_achievement(&statistic, &log.value, goal_for_day)?;
                let color = percentage_to_color(percentage);

                // Convert goal to DrawableGoal if present
                let drawable_goal = goal_for_day
                    .map(|g| json_to_drawable_goal(&statistic, g))
                    .transpose()?;

                // Create the data point with all processed data
                Ok(DataPoint::new(
                    log.date,
                    drawable_value,
                    color,
                    drawable_goal,
                ))
            })
            .collect();

        let points = points?;

        // 8. Calculate the Y-axis configuration using domain logic
        let y_axis = YAxis::from_data_points(&points, plot_spec.plot_type)?;

        // 9. Return the assembled PlotData
        Ok(PlotData::new(
            plot_spec.title.clone(),
            points,
            date_range,
            plot_spec.plot_type,
            y_axis,
        ))
    }

    /// Generates a rendered plot image for a given plot ID, date range, and render configuration
    pub fn generate_plot_image(
        &self,
        plot_id: PlotId,
        date_range: DateRange,
        config: RenderConfig,
    ) -> Result<Vec<u8>> {
        // 1. Get the plot data
        let plot_data = self.get_plot_data(plot_id, date_range)?;

        // 2. Render using the injected renderer
        self.renderer.render(&plot_data, &config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_domain::plot::data::DrawableData;
    use lifestats_domain::plot::specification::{PlotSpecification, PlotType};
    use lifestats_domain::shared::dated_value::DatedValue;
    use lifestats_domain::shared::ids::StatisticId;
    use lifestats_domain::shared::time_frame_list::TimeFrameList;
    use lifestats_domain::statistic::any_statistic::AnyStatistic;
    use lifestats_domain::statistic::types::logable_statistics::{
        integer_amount::IntegerAmountStatistic, time_spans::TimeSpansStatistic,
    };
    use lifestats_domain::statistic::value_objects::goals::amount_goal::AmountGoal;
    use lifestats_domain::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use serde_json::{json, Value};

    // We'll use mockall to create mock implementations
    use mockall::mock;

    mock! {
        PlotRepo {}
        impl PlotRepository for PlotRepo {
            fn find_by_id(&self, id: PlotId) -> Result<Option<PlotSpecification>>;
            fn find_all(&self) -> Result<Vec<PlotSpecification>>;
            fn find_by_statistic_id(&self, statistic_id: StatisticId) -> Result<Vec<PlotSpecification>>;
            fn save(&self, plot_spec: &PlotSpecification) -> Result<()>;
            fn delete(&self, id: PlotId) -> Result<()>;
        }
    }

    mock! {
        StatRepo {}
        impl StatisticRepository for StatRepo {
            fn find_metadata_by_id(&self, id: StatisticId) -> Result<lifestats_domain::statistic::any_statistic::AnyStatistic>;
            fn find_all_metadata(&self) -> Result<Vec<lifestats_domain::statistic::any_statistic::AnyStatistic>>;
            fn save_metadata(&self, statistic: &lifestats_domain::statistic::any_statistic::AnyStatistic) -> Result<()>;
            fn find_log_value(&self, id: StatisticId, date: NaiveDate) -> Result<Option<Value>>;
            fn save_log_value(&self, id: StatisticId, date: NaiveDate, value: Value) -> Result<()>;
            fn find_log_values_in_range(&self, id: StatisticId, range: DateRange) -> Result<Vec<DatedValue<Value>>>;
            fn find_goal(&self, id: StatisticId, date: NaiveDate) -> Result<Option<Value>>;
            fn save_goal(&self, id: StatisticId, date: NaiveDate, goal: Option<Value>) -> Result<()>;
            fn find_goals_in_range(&self, id: StatisticId, range: DateRange) -> Result<Vec<DatedValue<Value>>>;
            fn find_default_goal(&self, id: StatisticId) -> Result<Option<Value>>;
            fn save_default_goal(&self, id: StatisticId, goal: Value) -> Result<()>;
            fn delete_default_goal(&self, id: StatisticId) -> Result<()>;
        }
    }

    mock! {
        Renderer {}
        impl PlotRenderer for Renderer {
            fn render(&self, data: &PlotData, config: &RenderConfig) -> Result<Vec<u8>>;
        }
    }

    #[test]
    fn test_get_plot_data_success() {
        // Setup mocks
        let mut stat_repo = MockStatRepo::new();
        let mut plot_repo = MockPlotRepo::new();
        let renderer = MockRenderer::new();

        let plot_id = PlotId::from(1);
        let stat_id = StatisticId::from(42);
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
        let date_range = DateRange::new(start, end).unwrap();

        // Configure plot repository mock
        let plot_spec = PlotSpecification::new(
            plot_id,
            "Test Plot".to_string(),
            stat_id,
            PlotType::LineChart,
        );
        plot_repo
            .expect_find_by_id()
            .withf(move |id| *id == plot_id)
            .returning(move |_| Ok(Some(plot_spec.clone())));

        // Create a test statistic
        let test_stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(stat_id, "Test Category".into(), "Test Stat".into()),
            None,
            start,
        ));

        // Configure statistic repository mock
        stat_repo
            .expect_find_metadata_by_id()
            .withf(move |id| *id == stat_id)
            .returning(move |_| Ok(test_stat.clone()));

        stat_repo
            .expect_find_log_values_in_range()
            .withf(move |id, _range| *id == stat_id)
            .returning(|_, _| {
                Ok(vec![DatedValue::new(
                    NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    Value::from(10),
                )])
            });

        stat_repo
            .expect_find_goals_in_range()
            .withf(move |id, _range| *id == stat_id)
            .returning(|_, _| Ok(vec![]));

        // Create service
        let service =
            PlottingService::new(Arc::new(stat_repo), Arc::new(plot_repo), Arc::new(renderer));

        // Test
        let result = service.get_plot_data(plot_id, date_range);
        assert!(result.is_ok());
        let plot_data = result.unwrap();
        assert_eq!(plot_data.title, "Test Plot");
        assert_eq!(plot_data.points.len(), 1);
    }

    #[test]
    fn test_get_plot_data_not_found() {
        let stat_repo = MockStatRepo::new();
        let mut plot_repo = MockPlotRepo::new();
        let renderer = MockRenderer::new();

        let plot_id = PlotId::from(999);
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
        let date_range = DateRange::new(start, end).unwrap();

        plot_repo.expect_find_by_id().returning(|_| Ok(None));

        let service =
            PlottingService::new(Arc::new(stat_repo), Arc::new(plot_repo), Arc::new(renderer));
        let result = service.get_plot_data(plot_id, date_range);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_plot_data_for_time_frames_plot_produces_correct_drawable_data() {
        // ARRANGE
        let mut stat_repo = MockStatRepo::new();
        let mut plot_repo = MockPlotRepo::new();
        let renderer = MockRenderer::new(); // Not used, but needed for construction

        let plot_id = PlotId::from(7);
        let stat_id = StatisticId::from(10);
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
        let date_range = DateRange::new(start, end).unwrap();

        // 1. Configure PlotRepository to return a TimeFrames plot spec
        let plot_spec = PlotSpecification::new(
            plot_id,
            "Study Time Plot".to_string(),
            stat_id,
            PlotType::TimeFrames,
        );
        plot_repo
            .expect_find_by_id()
            .returning(move |_| Ok(Some(plot_spec.clone())));

        // 2. Configure StatisticRepository to return a TimeSpansStatistic...
        let test_stat = AnyStatistic::TimeSpans(TimeSpansStatistic::new(
            StatisticMetadata::new(stat_id, "Productivity".into(), "Study Time".into()),
            None,
            start,
        ));
        stat_repo
            .expect_find_metadata_by_id()
            .returning(move |_| Ok(test_stat.clone()));

        // 3. ...and a logged value for it.
        let time_frame_list_json = json!([
            {"start": "09:00:00", "end": "10:00:00"}
        ]);
        let expected_time_frames: TimeFrameList =
            serde_json::from_value(time_frame_list_json.clone()).unwrap();
        stat_repo
            .expect_find_log_values_in_range()
            .returning(move |_, _| {
                Ok(vec![DatedValue::new(
                    NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    time_frame_list_json.clone(),
                )])
            });

        // This mock will not be hit, but is required by the function signature
        stat_repo
            .expect_find_goals_in_range()
            .returning(|_, _| Ok(vec![]));

        let service =
            PlottingService::new(Arc::new(stat_repo), Arc::new(plot_repo), Arc::new(renderer));

        // ACT
        let result = service.get_plot_data(plot_id, date_range);
        assert!(result.is_ok());
        let plot_data = result.unwrap();

        // ASSERT
        assert!(
            !plot_data.points.is_empty(),
            "Should have produced data points"
        );

        let first_point = &plot_data.points[0];

        // This is the key assertion that will fail before the fix.
        // We expect TimeFrames data, but the buggy logic will produce Integer data.
        assert!(
            matches!(first_point.value, DrawableData::TimeFrames(_)),
            "For a TimeFrames plot, DrawableData should be TimeFrames, but it was {:?} instead.",
            first_point.value
        );

        if let DrawableData::TimeFrames(ref frames) = first_point.value {
            assert_eq!(frames, &expected_time_frames);
        }

        if let DrawableData::Integer(val) = first_point.value {
            panic!(
                "Incorrectly converted TimeFrameList to Integer({}). This is the root cause of the panic.",
                val
            );
        }
    }

    #[test]
    fn test_get_plot_data_uses_default_goal_when_no_daily_goal_is_present() {
        // ARRANGE
        let mut stat_repo = MockStatRepo::new();
        let mut plot_repo = MockPlotRepo::new();
        let renderer = MockRenderer::new(); // Not used, but needed for construction

        let plot_id = PlotId::from(1);
        let stat_id = StatisticId::from(42);
        let test_date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let date_range = DateRange::new(test_date, test_date).unwrap();

        // 1. Configure PlotRepository to return a simple plot spec
        let plot_spec = PlotSpecification::new(
            plot_id,
            "Test Plot With Default Goal".to_string(),
            stat_id,
            PlotType::LineChart,
        );
        plot_repo
            .expect_find_by_id()
            .returning(move |_| Ok(Some(plot_spec.clone())));

        // 2. Configure StatisticRepository to return a statistic WITH a default goal
        let default_goal = AmountGoal::new(100, true);
        let test_stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(stat_id, "Testing".into(), "Test Stat".into()),
            Some(default_goal.clone()),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(), // Goal is valid from this date
        ));
        stat_repo
            .expect_find_metadata_by_id()
            .returning(move |_| Ok(test_stat.clone()));

        // 3. Return a log value for the date
        stat_repo
            .expect_find_log_values_in_range()
            .returning(move |_, _| {
                Ok(vec![DatedValue::new(
                    test_date,
                    json!(75), // A value below the goal
                )])
            });

        // 4. CRITICAL: Return NO daily goals. This is the bug condition.
        stat_repo
            .expect_find_goals_in_range()
            .returning(|_, _| Ok(vec![]));

        let service =
            PlottingService::new(Arc::new(stat_repo), Arc::new(plot_repo), Arc::new(renderer));

        // ACT
        let result = service.get_plot_data(plot_id, date_range);

        // ASSERT
        assert!(result.is_ok(), "get_plot_data should not fail");
        let plot_data = result.unwrap();
        assert_eq!(plot_data.points.len(), 1, "Should have one data point");

        let point = &plot_data.points[0];

        // This assertion will FAIL with the current code, as `point.goal` will be `None`.
        // It will PASS after applying the fix.
        use lifestats_domain::plot::data::DrawableGoal;
        let expected_goal = DrawableGoal::Integer(default_goal.target_amount());
        assert_eq!(
            point.goal,
            Some(expected_goal),
            "The data point's goal should be the statistic's default goal"
        );

        // This assertion also verifies the logic by checking the resulting color.
        // 75 is < 100, so the color should be red. With the bug, the color would be gray [128, 128, 128].
        assert_eq!(
            point.color,
            [245, 169, 142],
            "Color should be red because the value (75) is below the default goal (100)"
        );
    }
}
