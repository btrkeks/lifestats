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

pub struct CircularProgressBarPlot {
    pub metadata: PlotMetadata,
    pub statistic_id: StatisticId,
    pub size: (usize, usize),
    pub statistic_repository: Arc<dyn StatisticRepositoryTrait>,
    pub backend: Arc<dyn PlottingBackend>,
}

impl Plot for CircularProgressBarPlot {
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

    fn plot(&self, start_date: NaiveDate, _end_date: NaiveDate) -> anyhow::Result<Image> {
        let stat = self
            .statistic_repository
            .get_by_id(self.get_statistic_id())?;
        let integer_stat = stat.into_integer_statistic()?;

        let value = integer_stat
            .get_integer_data_closed_range(start_date, start_date)?
            .first()
            .and_then(|d| d.value)
            .unwrap_or(0);

        let goal = integer_stat
            .get_integer_goals_closed_range(start_date, start_date)?
            .first()
            .and_then(|d| d.value)
            .unwrap_or(0);

        let score = integer_stat
            .get_scores_for_closed_range(start_date, start_date)?
            .first()
            .and_then(|d| d.value)
            .unwrap_or(0.0);

        // Pass the extracted data to the backend for plotting
        self.backend.circular_progress_plot(
            self.get_title().to_string(),
            self.size,
            value,
            goal,
            percent_to_color(Some(score)),
        )
    }
}
