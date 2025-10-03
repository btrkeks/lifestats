use crate::plot::objects::plot_data::PlotDataPoint;
use crate::plot::plot_backends::plotting_backend::PlottingBackend;
use crate::plot::plot_config::PlotConfig;
use crate::plot::plot_metadata::PlotMetadata;
use crate::plot::traits::plot::Plot;
use crate::plot::util::percent_to_color::percent_to_color;
use crate::statistic::entities::logable_statistics::time_spans::TimeSpansStatistic;
use log::debug;
use std::sync::Arc;
use transfer_objects::image::Image;
use crate::statistic::traits::base_statistic::Statistic;

pub struct TimeFramesPlot {
    pub metadata: PlotMetadata,
    pub statistic: Arc<TimeSpansStatistic>,
    pub plotting_backend: Arc<dyn PlottingBackend>,
}

impl Plot for TimeFramesPlot {
    fn get_metadata(&self) -> &PlotMetadata {
        &self.metadata
    }

    fn plot(&self, plot_config: PlotConfig) -> anyhow::Result<Image> {
        debug!("Plotting TimeFramesPlot: {}", self.metadata.title);

        let start_date = plot_config.date_range.start();
        let end_date = plot_config.date_range.end();
        
        let data = self.statistic.get_log_values_closed_range(start_date, end_date);
        let goals = self.statistic.get_goals_closed_range(start_date, end_date);
        let scores = self.statistic.get_scores_closed_range(start_date, end_date)?;
        let mean_score = if scores.is_empty() { None } else {
            Some(scores.iter().map(|s| s.value.as_num()).sum::<i64>() / scores.len() as i64)
        };
        let streak = self.statistic.get_streak()?;

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
            plot_config.size,
            plot_data,
            "Time".to_string(),
            percent_to_color(mean_score),
            start_date,
            end_date,
            streak,
        )
    }
}
