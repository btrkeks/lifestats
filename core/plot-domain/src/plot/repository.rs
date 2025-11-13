use crate::plot::specification::PlotSpecification;
use lifestats_core::shared::ids::{PlotId, StatisticId};
use lifestats_core::statistic::ports::error::RepositoryError;

/// Port (trait) for persisting plot configurations
///
/// # ⚠️ DEPRECATION NOTICE
/// **This trait will be removed in Phase 4 of the plotting refactor.**
///
/// The plotting system no longer requires storing plot configurations. The new
/// stateless architecture generates plots on-the-fly without needing a plot repository.
///
/// **Migration Guide:**
/// - Old approach: Store PlotSpecification in database → load via PlotRepository
/// - New approach: Accept parameters directly (statistic_id, plot_type, date_range)
///
/// **Impact:**
/// - The `plots` database table will be dropped
/// - `SqlitePlotRepository` implementation will be removed
/// - Application layer will no longer depend on this trait
///
/// See the TODO.md file for complete refactoring details.
///
/// **Note:** Rust does not support deprecating traits, so this is documented only.
/// All usages should be migrated to the new stateless plotting approach.
#[cfg_attr(feature = "testing", mockall::automock)]
pub trait PlotRepository: Send + Sync {
    /// Finds a plot specification by its ID
    fn find_by_id(&self, id: PlotId) -> Result<Option<PlotSpecification>, RepositoryError>;

    /// Retrieves all plot specifications
    fn find_all(&self) -> Result<Vec<PlotSpecification>, RepositoryError>;

    /// Finds all plot specifications for a given statistic
    fn find_by_statistic_id(
        &self,
        statistic_id: StatisticId,
    ) -> Result<Vec<PlotSpecification>, RepositoryError>;

    /// Saves (creates or updates) a plot specification
    fn save(&self, plot_spec: &PlotSpecification) -> Result<(), RepositoryError>;

    /// Deletes a plot specification by its ID
    fn delete(&self, id: PlotId) -> Result<(), RepositoryError>;
}
