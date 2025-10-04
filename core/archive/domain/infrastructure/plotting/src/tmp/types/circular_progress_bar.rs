use crate::plot::plot_backends::plotting_backend::PlottingBackend;
use crate::plot::plot_config::PlotConfig;
use crate::plot::plot_metadata::PlotMetadata;
use crate::plot::traits::plot::Plot;
use crate::plot::util::percent_to_color::percent_to_color;
use crate::statistic::repositories::statistic_repository::StatisticRepository;
use std::sync::Arc;
use transfer_objects::image::Image;
use transfer_objects::statistic_id::StatisticId;

pub struct CircularProgressBarPlot {
    pub metadata: PlotMetadata,
    pub statistic_id: StatisticId,
    pub size: (usize, usize),
    pub statistic_repository: Arc<dyn StatisticRepository>,
    pub backend: Arc<dyn PlottingBackend>,
}

impl Plot for CircularProgressBarPlot {
    fn get_metadata(&self) -> &PlotMetadata {
        &self.metadata
    }

    fn plot(&self, plot_config: PlotConfig) -> anyhow::Result<Image> {
        // TODO: Implement proper circular progress plot
        // This is a stub implementation to get compilation working
        // The repository interface needs to be updated to support general statistic loading
        
        let value = 0;
        let goal = 100;
        let score = 0.0;

        // Pass stub data to the backend for plotting
        self.backend.circular_progress_plot(
            self.get_title().to_string(),
            plot_config.size,
            value,
            goal,
            percent_to_color(Some(score)),
        )
    }
}
