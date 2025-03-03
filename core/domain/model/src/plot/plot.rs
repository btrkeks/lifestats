use crate::plot::objects::plot_metadata::PlotMetadata;
use anyhow::Result;
use chrono::NaiveDate;
use transfer_objects::image::Image;
use transfer_objects::statistic_id::{PlotId, StatisticId};

pub trait Plot {
    type PointType;
    fn get_width(&self) -> usize;
    fn get_height(&self) -> usize;
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

    fn contains_statistic_with_id(&self, id: StatisticId) -> bool;

    fn plot(&self, start_date: NaiveDate, end_date: NaiveDate) -> Result<Image>;
}
