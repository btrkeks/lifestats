use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::dto::{PlotDataResult, PlotPointDto};
use crate::error::ApplicationResult;
use anyhow::anyhow;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::value_objects::goals::goal_trait::Goal;
use plot_domain::plot::repository::PlotRepository;

use super::query::GetPlotDataQuery;

/// Handler for GetPlotDataQuery
///
/// This handler returns PURE DATA without any presentation logic.
/// It computes goal achievement percentages (business logic) but does NOT:
/// - Convert percentages to colors (presentation)
/// - Convert JSON to DrawableData (presentation)
/// - Apply any rendering transformations (presentation)
pub struct GetPlotDataQueryHandler<S, P>
where
    S: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
{
    statistic_repository: Arc<S>,
    plot_repository: Arc<P>,
}

impl<S, P> GetPlotDataQueryHandler<S, P>
where
    S: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
{
    pub fn new(statistic_repository: Arc<S>, plot_repository: Arc<P>) -> Self {
        Self {
            statistic_repository,
            plot_repository,
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

impl<S, P> QueryHandler<GetPlotDataQuery> for GetPlotDataQueryHandler<S, P>
where
    S: StatisticRepository + Send + Sync + 'static,
    P: PlotRepository + Send + Sync + 'static,
{
    fn handle(&self, query: GetPlotDataQuery) -> ApplicationResult<PlotDataResult> {
        // 1. Load the plot specification
        let plot_spec = self
            .plot_repository
            .find_by_id(query.plot_id)?
            .ok_or_else(|| anyhow!("Plot not found: {}", query.plot_id))?;

        // 2. Extract the statistic ID
        let statistic_id = plot_spec.statistic_id;

        // 3. Load statistic metadata
        let statistic = self
            .statistic_repository
            .find_metadata_by_id(statistic_id)?;

        // 4. Load log values and per-day goals for the date range
        let log_values = self
            .statistic_repository
            .find_log_values_in_range(statistic_id, query.date_range)?;

        let daily_goals = self
            .statistic_repository
            .find_goals_in_range(statistic_id, query.date_range)?;

        // 5. Create a map of dates to goal values for efficient lookup
        let daily_goal_map: std::collections::HashMap<_, _> = daily_goals
            .into_iter()
            .map(|g| (g.date, g.value.clone()))
            .collect();

        // 6. Get the default goal and the date it's valid from
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

        // 7. Convert log values to PlotPointDto (pure data, no rendering)
        let points: ApplicationResult<Vec<PlotPointDto>> = log_values
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

                // Create the plot point with RAW data (no colors, no drawable types)
                Ok(PlotPointDto::new(
                    log.date,
                    log.value,             // Raw JSON value
                    goal_for_day.cloned(), // Raw JSON goal
                    percentage,            // Computed percentage
                ))
            })
            .collect();

        let points = points?;

        // 8. Return pure data result
        Ok(PlotDataResult {
            title: plot_spec.title.clone(),
            points,
            date_range: query.date_range,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_core::shared::date_range::DateRange;
    use lifestats_core::shared::dated_value::DatedValue;
    use lifestats_core::shared::ids::{PlotId, StatisticId};
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::value_objects::goals::amount_goal::AmountGoal;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
    use plot_domain::plot::repository::MockPlotRepository;
    use plot_domain::plot::specification::{PlotSpecification, PlotType};
    use serde_json::json;

    #[test]
    fn test_get_plot_data_returns_pure_data() {
        // Arrange
        let mut stat_repo = MockStatisticRepository::new();
        let mut plot_repo = MockPlotRepository::new();

        let plot_id = PlotId::from(1);
        let stat_id = StatisticId::from(42);
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
        let date_range = DateRange::new(start, end).unwrap();

        // Configure plot repository
        let plot_spec = PlotSpecification::new(
            plot_id,
            "Test Plot".to_string(),
            stat_id,
            PlotType::LineChart,
        );
        plot_repo
            .expect_find_by_id()
            .returning(move |_| Ok(Some(plot_spec.clone())));

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
                    json!(15), // Achieves goal of 10
                )])
            });

        stat_repo
            .expect_find_goals_in_range()
            .returning(|_, _| Ok(vec![]));

        let handler = GetPlotDataQueryHandler::new(Arc::new(stat_repo), Arc::new(plot_repo));
        let query = GetPlotDataQuery {
            plot_id,
            date_range,
        };

        // Act
        let result = handler.handle(query);

        // Assert
        assert!(result.is_ok());
        let plot_data = result.unwrap();
        assert_eq!(plot_data.title, "Test Plot");
        assert_eq!(plot_data.points.len(), 1);

        // Verify the point contains RAW data (not colors or drawable types)
        let point = &plot_data.points[0];
        assert_eq!(point.value, json!(15));
        assert!(point.percentage_achieved.is_some());
        // The percentage should be > 100% since 15 > 10
        assert!(point.percentage_achieved.unwrap().as_num() >= 100);
    }
}
