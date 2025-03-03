use crate::plot::objects::plot_data::PlotDataPoint;
use crate::plot::objects::plot_metadata::PlotMetadata;
use crate::plot::plot::Plot;
use crate::plot::plot_backends::plotting_backend::PlottingBackend;
use crate::plot::util::percent_to_color::percent_to_color;
use crate::repositories::statistic_repository::StatisticRepositoryTrait;
use anyhow::Result;
use chrono::NaiveDate;
use log::debug;
use std::sync::Arc;
use transfer_objects::image::Image;
use transfer_objects::statistic_id::StatisticId;
use crate::statistic::integer_statistic::IntegerStatistic;
use crate::statistic::statistic::StatisticBase;

pub struct BarPlot {
    pub metadata: PlotMetadata,
    pub statistic_id: StatisticId,
    pub size: (usize, usize),
    pub statistic_repository: Arc<dyn StatisticRepositoryTrait>,
    pub plotting_backend: Arc<dyn PlottingBackend>,
}

impl BarPlot {
    pub fn new(
        metadata: PlotMetadata,
        statistic_id: StatisticId,
        size: (usize, usize),
        statistic_repository: impl StatisticRepositoryTrait + 'static,
        backend: impl PlottingBackend + 'static,
    ) -> BarPlot {
        Self {
            metadata,
            statistic_id,
            size,
            statistic_repository: Arc::new(statistic_repository),
            plotting_backend: Arc::new(backend),
        }
    }
}

impl Plot for BarPlot {
    type PointType = ();

    fn get_width(&self) -> usize {
        self.size.0
    }

    fn get_height(&self) -> usize {
        self.size.1
    }

    fn get_metadata(&self) -> &PlotMetadata {
        &self.metadata
    }

    fn contains_statistic_with_id(&self, id: StatisticId) -> bool {
        self.statistic_id == id
    }

    fn plot(&self, start_date: NaiveDate, end_date: NaiveDate) -> Result<Image> {
        debug!("Plotting bar plot: {}", self.metadata.title);

        let stat = self
            .statistic_repository
            .get_by_id(self.get_statistic_id())?;
        let integer_stat = stat.into_integer_statistic()?;

        // TODO: Probably refactor this
        let values = integer_stat.get_integer_data_closed_range(start_date, end_date)?;
        let goals = integer_stat.get_integer_goals_closed_range(start_date, end_date)?;
        let scores = integer_stat.get_scores_for_closed_range(start_date, end_date)?;
        let mean_score = integer_stat.get_mean_score_for_closed_range(start_date, end_date)?;
        let streak = integer_stat.get_streak()?;

        let plot_points = values
            .into_iter()
            .zip(goals)
            .zip(scores)
            .map(|((value, goal), score)| {
                debug_assert_eq!(value.date, goal.date);
                debug_assert_eq!(value.date, score.date);
                PlotDataPoint {
                    date: value.date,
                    value: value.value,
                    goal: goal.value,
                    color: percent_to_color(score.value),
                }
            })
            .collect();

        self.plotting_backend.bar_plot(
            self.get_title(),
            self.size,
            plot_points,
            "",
            percent_to_color(mean_score),
            integer_stat.get_y_axis_conversion(),
            start_date,
            end_date,
            streak,
        )
    }
}
