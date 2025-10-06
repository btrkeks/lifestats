use crate::shared::date_range::DateRange;
use crate::shared::dated_value::DatedValue;
use crate::shared::ids::StatisticId;
use crate::statistic::any_statistic::AnyStatistic;
use anyhow::Result;
use chrono::NaiveDate;

#[cfg_attr(any(test, feature = "testing"), mockall::automock)]
pub trait StatisticRepository {
    // --- Metadata Operations ---
    fn find_metadata_by_id(&self, id: StatisticId) -> Result<AnyStatistic>;
    fn find_all_metadata(&self) -> Result<Vec<AnyStatistic>>;
    fn save_metadata(&self, statistic: &AnyStatistic) -> Result<()>;

    // --- Log Value Operations ---
    fn find_log_value(&self, id: StatisticId, date: NaiveDate)
        -> Result<Option<serde_json::Value>>;
    fn save_log_value(
        &self,
        id: StatisticId,
        date: NaiveDate,
        value: serde_json::Value,
    ) -> Result<()>;
    fn find_log_values_in_range(
        &self,
        id: StatisticId,
        range: DateRange,
    ) -> Result<Vec<DatedValue<serde_json::Value>>>;

    // --- Goal Operations ---
    fn find_goal(&self, id: StatisticId, date: NaiveDate) -> Result<Option<serde_json::Value>>;
    fn save_goal(
        &self,
        id: StatisticId,
        date: NaiveDate,
        goal: Option<serde_json::Value>,
    ) -> Result<()>;
    fn find_goals_in_range(
        &self,
        id: StatisticId,
        range: DateRange,
    ) -> Result<Vec<DatedValue<serde_json::Value>>>;
    fn find_default_goal(&self, id: StatisticId) -> Result<Option<serde_json::Value>>;
    fn save_default_goal(&self, id: StatisticId, goal: serde_json::Value) -> Result<()>;
    fn delete_default_goal(&self, id: StatisticId) -> Result<()>;
}
