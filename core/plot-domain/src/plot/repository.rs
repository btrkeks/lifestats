use crate::plot::specification::PlotSpecification;
use anyhow::Result;
use lifestats_core::shared::ids::{PlotId, StatisticId};

/// Port (trait) for persisting plot configurations
/// This keeps the domain independent of specific database implementations
#[cfg_attr(feature = "testing", mockall::automock)]
pub trait PlotRepository: Send + Sync {
    /// Finds a plot specification by its ID
    fn find_by_id(&self, id: PlotId) -> Result<Option<PlotSpecification>>;

    /// Retrieves all plot specifications
    fn find_all(&self) -> Result<Vec<PlotSpecification>>;

    /// Finds all plot specifications for a given statistic
    fn find_by_statistic_id(&self, statistic_id: StatisticId) -> Result<Vec<PlotSpecification>>;

    /// Saves (creates or updates) a plot specification
    fn save(&self, plot_spec: &PlotSpecification) -> Result<()>;

    /// Deletes a plot specification by its ID
    fn delete(&self, id: PlotId) -> Result<()>;
}
