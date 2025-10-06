use crate::plot::integer_plot_data::IntegerPlotData;
use crate::plot::plot_backends::plotting_backend::PlottingBackend;
use crate::plot::plot_config::PlotConfig;
use crate::plot::plot_metadata::PlotMetadata;
use crate::plot::traits::plot::Plot;
use crate::statistic::traits::integer_statistic::IntegerStatistic;
use crate::value_objects::seconds::Seconds;
use anyhow::Result;
use std::sync::Arc;
use transfer_objects::image::Image;

pub struct BarPlot {
    pub metadata: PlotMetadata,

    pub statistic: Arc<dyn IntegerStatistic<GoalType=Seconds, LogValueType=Seconds>>, // TODO remove associated types

    pub plotting_backend: Arc<dyn PlottingBackend>,
}

impl Plot for BarPlot {
    fn get_metadata(&self) -> &PlotMetadata {
        &self.metadata
    }

    fn plot(&self, plot_config: PlotConfig) -> Result<Image> {
        self.plotting_backend.bar_plot(
            IntegerPlotData::from_integer_statistic(
                self.get_title().to_string(),
                plot_config.date_range,
                self.statistic.clone()
            )?
        )
    }
}
