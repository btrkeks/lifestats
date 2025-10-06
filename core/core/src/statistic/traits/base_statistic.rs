use crate::shared::ids::StatisticId;
use crate::statistic::error::Result;
use crate::statistic::value_objects::goals::goal_trait::Goal;
use crate::statistic::value_objects::statistic_metadata::StatisticMetadata;
use chrono::NaiveDate;

pub trait Statistic {
    type LogValueType;
    type GoalType: Goal<LogValue = Self::LogValueType>;

    //===========================================================================
    // METADATA
    //===========================================================================

    fn get_metadata(&self) -> &StatisticMetadata;

    fn id(&self) -> StatisticId {
        self.get_metadata().id()
    }

    fn get_category(&self) -> &str {
        self.get_metadata().category()
    }

    fn get_name(&self) -> &str {
        self.get_metadata().name()
    }

    //===========================================================================
    // DEFAULT GOAL MANAGEMENT
    //===========================================================================

    /// Returns the current default goal
    fn get_default_goal(&self) -> Option<&Self::GoalType>;

    /// Sets a new default goal `goal` starting at `start_date` (inclusive).
    /// This function shouldn't be used by user code as it isn't supposed to flush the previous goal
    fn _set_default_goal(&mut self, goal: Self::GoalType, start_date: NaiveDate);

    /// Updates the in-memory default goal metadata.
    ///
    /// The application layer is responsible for applying any persistence
    /// side-effects (like flushing per-day goals) when the default goal
    /// changes.
    fn set_new_default_goal(&mut self, goal: Self::GoalType, start_date: NaiveDate) -> Result<()> {
        self._set_default_goal(goal, start_date);
        Ok(())
    }

    /// Returns the date from which the current default goal became active (including)
    fn get_default_goal_valid_since(&self) -> NaiveDate;
}
