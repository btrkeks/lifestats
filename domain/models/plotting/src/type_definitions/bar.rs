use statistics::objects::statistic_metadata::StatisticId;
use statistics::repositories::statistic_definition_repository::StatisticDefinitionRepository;
use crate::backends::plotting_backend::PlottingBackend;
use crate::objects::image::Image;
use crate::objects::plot_metadata::PlotMetadata;
use crate::plot::Plot;
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct BarPlot {
    metadata: PlotMetadata,
    statistic_id: StatisticId,
}

impl Plot for BarPlot {
    fn get_metadata(&self) -> &PlotMetadata {
        &self.metadata
    }

    fn contains_statistic_with_id(&self, id: StatisticId) -> bool {
        self.statistic_id == id
    }

    fn plot(&self,
            statistic_repository: Box<dyn StatisticDefinitionRepository>,
            plotting_backend: Box<dyn PlottingBackend>) -> Result<Image> {
            
            // plotting_backend.barplot(self.get_title(), , )
            unimplemented!()
    }
}