pub mod plot_data;
pub mod plot_getter;
pub mod plot_types;

use crate::objects::image::Image;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use statistics::objects::statistic_metadata::StatisticId;
use statistics::repositories::statistic_data_repository::StatisticDataRepository;
use statistics::repositories::statistic_definition_repository::StatisticDefinitionRepository;
use crate::backends::plotting_backend::PlottingBackend;
use crate::objects::plot_metadata::{PlotId, PlotMetadata};
use crate::type_definitions::bar::BarPlot;
use crate::type_definitions::circular_progress_bar::CircularProgressBarPlot;
use crate::type_definitions::scatter::ScatterPlot;
use crate::type_definitions::time_frames::TimeFramesPlot;
use crate::type_definitions::time_line::TimeLinePlot;

pub trait Plot {
    fn get_metadata(&self) -> &PlotMetadata;

    fn get_id(&self) -> PlotId {
        self.get_metadata().id
    }

    fn get_title(&self) -> &str {
        &self.get_metadata().title
    }

    fn contains_statistic_with_id(&self, id: StatisticId) -> bool;

    fn plot(&self,
            statistic_repository: Box<dyn StatisticDefinitionRepository>,
            plotting_backend: Box<dyn PlottingBackend>) -> anyhow::Result<Image>;
}

#[derive(Debug, Clone)]
pub enum AnyPlot {
    Bar(BarPlot),
    CircularProgressBar(CircularProgressBarPlot),
    Scatter(ScatterPlot),
    TimeFrames(TimeFramesPlot),
    TimeLine(TimeLinePlot),
}