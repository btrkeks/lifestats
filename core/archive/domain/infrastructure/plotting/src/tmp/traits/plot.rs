use crate::plot::plot_config::PlotConfig;
use crate::plot::plot_metadata::PlotMetadata;
use anyhow::Result;
use transfer_objects::image::Image;
use transfer_objects::statistic_id::{PlotId, StatisticId};

pub trait Plot {

    //===========================================================================
    // METADATA ACCESS AND DERIVED PROPERTIES
    //===========================================================================

    fn get_metadata(&self) -> &PlotMetadata;

    fn get_id(&self) -> PlotId {
        self.get_metadata().id
    }

    fn get_title(&self) -> &str {
        &self.get_metadata().title
    }

    fn get_statistic_id(&self) -> StatisticId {
        self.get_metadata().statistic_id
    }

    //===========================================================================
    // PLOT GENERATION
    //===========================================================================

    fn plot(&self, plot_config: PlotConfig) -> Result<Image>;
}
