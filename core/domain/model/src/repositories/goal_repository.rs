use crate::statistic::objects::goal::Goal;
use crate::statistic::objects::logging_data::DateValue;
use anyhow::Result;
use chrono::NaiveDate;
use transfer_objects::statistic_id::StatisticId;

pub trait GoalRepositoryTrait: Sync + Send {
    type StoredValue;
    type GoalType: Goal<Self::StoredValue>;

    // TODO: Other code is relying on the fact that data that is not in the database isn't included
    //       This is needed to distinguish between explicitly set to None or not set at all.
    /// Returns the goal for each date in the closed range [start_date, end_date] in ASCENDING ORDER.
    fn get_for_closed_range(
        &self,
        id: StatisticId,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<DateValue<Option<Self::GoalType>>>>;

    /// The outer Option indicates whether the value is contained in the database
    /// The inner Option indicates whether the value is explicitly set to None
    fn get(&self, id: StatisticId, date: NaiveDate) -> Result<Option<Option<Self::GoalType>>>;

    fn set_for_date(
        &self,
        id: StatisticId,
        date: NaiveDate,
        new_goal: Option<Self::GoalType>,
    ) -> Result<()>;

    fn get_default(&self, id: StatisticId) -> Result<Option<Self::GoalType>>;

    fn set_default(&self, id: StatisticId, new_goal: Self::GoalType) -> Result<()>;

    fn delete_default(&self, id: StatisticId) -> Result<()>;
}
