use std::sync::Arc;

use anyhow::{anyhow, Result};
use lifestats_domain::plot::axis::YAxis;
use lifestats_domain::plot::data::{DataPoint, PlotData};
use lifestats_domain::plot::renderer::{PlotRenderer, RenderConfig};
use lifestats_domain::plot::repository::PlotRepository;
use lifestats_domain::shared::date_range::DateRange;
use lifestats_domain::shared::ids::PlotId;
use lifestats_domain::statistic::ports::statistic_repository::StatisticRepository;

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

        // 3. Load log values and goals for the date range
        let log_values = self
            .statistic_repository
            .find_log_values_in_range(statistic_id, date_range)?;

        let goals = self
            .statistic_repository
            .find_goals_in_range(statistic_id, date_range)?;

        // 4. Assemble into PlotData
        // Create a map of dates to goal values for efficient lookup
        let goal_map: std::collections::HashMap<_, _> = goals
            .into_iter()
            .map(|g| (g.date, g.value.clone()))
            .collect();

        // Convert log values to data points
        let points: Vec<DataPoint> = log_values
            .into_iter()
            .map(|log| {
                let goal = goal_map.get(&log.date).cloned();
                DataPoint::new(log.date, log.value.clone(), goal)
            })
            .collect();

        // 5. Calculate the Y-axis configuration using domain logic
        let y_axis = YAxis::from_data_points(&points, plot_spec.plot_type)?;

        // 6. Return the assembled PlotData
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
    use lifestats_domain::plot::specification::{PlotSpecification, PlotType};
    use lifestats_domain::shared::dated_value::DatedValue;
    use lifestats_domain::shared::ids::StatisticId;
    use serde_json::Value;

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

        // Configure statistic repository mock
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
}
