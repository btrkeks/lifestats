use crate::plot::objects::plot_data::PlotDataPoint;
use crate::plot::objects::plot_metadata::PlotMetadata;
use crate::plot::plot::Plot;
use crate::plot::plot_backends::plotting_backend::PlottingBackend;
use crate::plot::util::percent_to_color::percent_to_color;
use crate::repositories::statistic_repository::StatisticRepositoryTrait;
use chrono::NaiveDate;
use std::sync::Arc;
use transfer_objects::image::Image;
use transfer_objects::statistic_id::StatisticId;
use crate::statistic::statistic::StatisticBase;
use crate::statistic::integer_statistic::IntegerStatistic;

pub struct ScatterPlot {
    pub metadata: PlotMetadata,
    pub statistic_id: StatisticId,
    pub size: (usize, usize),
    pub statistic_repository: Arc<dyn StatisticRepositoryTrait>,
    pub plotting_backend: Arc<dyn PlottingBackend>,
}

impl Plot for ScatterPlot {
    type PointType = i64;

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

    fn plot(&self, start_date: NaiveDate, end_date: NaiveDate) -> anyhow::Result<Image> {
        let stat = self.statistic_repository.get_by_id(self.statistic_id)?;
        let integer_stat = stat.into_integer_statistic()?;

        let data = integer_stat.get_integer_data_closed_range(start_date, end_date)?;
        let goals = integer_stat.get_integer_goals_closed_range(start_date, end_date)?;
        let scores = integer_stat.get_scores_for_closed_range(start_date, end_date)?;
        let mean_score = integer_stat.get_mean_score_for_closed_range(start_date, end_date)?;
        let streak = integer_stat.get_streak()?;

        let plot_points: Vec<_> = data
            .iter()
            .zip(goals)
            .zip(scores)
            .map(|((logging_data, goal), score)| {
                PlotDataPoint::new(
                    logging_data.date,
                    logging_data.value,
                    goal.value,
                    percent_to_color(score.value),
                )
            })
            .collect();

        self.plotting_backend.scatter_plot(
            self.get_title().to_string(),
            self.size,
            plot_points,
            "".to_string(),
            percent_to_color(mean_score),
            integer_stat.get_y_axis_conversion(),
            start_date,
            end_date,
            streak,
        )
    }
}
