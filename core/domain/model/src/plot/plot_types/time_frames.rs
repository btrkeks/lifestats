use crate::plot::objects::plot_data::PlotDataPoint;
use crate::plot::objects::plot_metadata::PlotMetadata;
use crate::plot::plot::Plot;
use crate::plot::plot_backends::plotting_backend::PlottingBackend;
use crate::plot::util::percent_to_color::percent_to_color;
use crate::repositories::statistic_repository::StatisticRepositoryTrait;
use crate::statistic::objects::goal::Goal;
use crate::statistic::objects::time_frame_list::TimeFrameList;
use crate::statistic::statistic::{Statistic, StatisticBase};
use crate::statistic::statistic_types::any_statistic::AnyStatistic;
use anyhow::anyhow;
use chrono::NaiveDate;
use log::debug;
use std::sync::Arc;
use transfer_objects::image::Image;
use transfer_objects::statistic_id::StatisticId;

pub struct TimeFramesPlot {
    pub metadata: PlotMetadata,
    pub statistic_id: StatisticId,
    pub size: (usize, usize),
    pub statistic_repository: Arc<dyn StatisticRepositoryTrait>,
    pub plotting_backend: Arc<dyn PlottingBackend>,
}

impl TimeFramesPlot {
    pub fn new(
        metadata: PlotMetadata,
        statistic_id: StatisticId,
        size: (usize, usize),
        statistic_repository: impl StatisticRepositoryTrait + 'static,
        backend: impl PlottingBackend + 'static,
    ) -> Self {
        Self {
            metadata,
            statistic_id,
            size,
            statistic_repository: Arc::new(statistic_repository),
            plotting_backend: Arc::new(backend),
        }
    }
}

impl Plot for TimeFramesPlot {
    type PointType = TimeFrameList;

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
        debug!("Plotting TimeFramesPlot: {}", self.metadata.title);

        let statistic = self.statistic_repository.get_by_id(self.statistic_id)?;

        if let AnyStatistic::TimeSpans(ts) = statistic {
            let data = ts._get_values_for_closed_range(start_date, end_date)?;
            let goals = ts._get_goals_for_closed_range(start_date, end_date)?;
            let scores = ts.get_scores_for_closed_range(start_date, end_date)?;
            let mean_score = ts.get_mean_score_for_closed_range(start_date, end_date)?;
            let streak = ts.get_streak()?;

            let plot_data: Vec<_> = data
                .into_iter()
                .zip(goals)
                .zip(scores)
                .map(|((logging_data, goal), score)| {
                    PlotDataPoint::new(
                        logging_data.date,
                        logging_data.value,
                        goal.value.map(|g| g.get_value().clone()), // TODO: Unnecessary clone
                        percent_to_color(score.value),
                    )
                })
                .collect();

            self.plotting_backend.time_frames_plot(
                self.get_title().to_string(),
                self.size,
                plot_data,
                "Time".to_string(),
                percent_to_color(mean_score),
                start_date,
                end_date,
                streak,
            )
        } else {
            Err(anyhow!("Statistic is not a TimeSpansStatistic"))
        }
    }
}
