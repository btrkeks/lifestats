use crate::plot::integer_plot_data::IntegerPlotData;
use crate::plot::plot_backends::plotting_backend::PlottingBackend;
use crate::plot::plot_config::PlotConfig;
use crate::plot::plot_metadata::PlotMetadata;
use crate::plot::traits::plot::Plot;
use crate::statistic::traits::integer_statistic::IntegerStatistic;
use crate::value_objects::seconds::Seconds;
use std::sync::Arc;
use transfer_objects::image::Image;

pub struct ScatterPlot {
    pub metadata: PlotMetadata,
    pub statistic: Arc<dyn IntegerStatistic<GoalType=Seconds, LogValueType=Seconds>>, // TODO remove associated types
    pub plotting_backend: Arc<dyn PlottingBackend>,
}

impl Plot for ScatterPlot {
    fn get_metadata(&self) -> &PlotMetadata {
        &self.metadata
    }

    fn plot(&self, plot_config: PlotConfig) -> anyhow::Result<Image> {
        self.plotting_backend.scatter_plot(
            IntegerPlotData::from_integer_statistic(
                self.get_title().to_string(),
                plot_config.date_range,
                self.statistic.clone()
            )?
        )
    }
}
