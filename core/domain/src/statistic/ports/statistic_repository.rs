use crate::shared::date_range::DateRange;
use crate::shared::dated_value::DatedValue;
use crate::shared::ids::StatisticId;
use crate::statistic::any_statistic::AnyStatistic;
use anyhow::Result;
use chrono::NaiveDate;

#[cfg_attr(test, mockall::automock)]
pub trait StatisticRepository {
    // --- Metadata Operations ---
    fn find_metadata_by_id(&self, id: StatisticId) -> Result<AnyStatistic>;
    fn find_all_metadata(&self) -> Result<Vec<AnyStatistic>>;
    fn save_metadata(&self, statistic: &AnyStatistic) -> Result<()>;

    // --- Log Value Operations ---
    fn find_log_value<V>(&self, id: StatisticId, date: NaiveDate) -> Result<Option<V>>
    where
        V: serde::de::DeserializeOwned + 'static;
    fn find_log_values_in_range<V>(
        &self,
        id: StatisticId,
        range: DateRange,
    ) -> Result<Vec<DatedValue<V>>>
    where
        V: serde::de::DeserializeOwned + 'static;
    fn save_log_value<V>(&self, id: StatisticId, date: NaiveDate, value: V) -> Result<()>
    where
        V: serde::Serialize + 'static;

    // --- Goal Operations ---
    fn find_goal<G>(&self, id: StatisticId, date: NaiveDate) -> Result<Option<G>>
    where
        G: serde::de::DeserializeOwned + 'static;
    fn find_goals_in_range<G>(
        &self,
        id: StatisticId,
        range: DateRange,
    ) -> Result<Vec<DatedValue<G>>>
    where
        G: serde::de::DeserializeOwned + 'static;
    fn save_goal<G>(&self, id: StatisticId, date: NaiveDate, goal: Option<G>) -> Result<()>
    where
        G: serde::Serialize + 'static;
    fn find_default_goal<G>(&self, id: StatisticId) -> Result<Option<G>>
    where
        G: serde::de::DeserializeOwned + 'static;
    fn save_default_goal<G>(&self, id: StatisticId, goal: G) -> Result<()>
    where
        G: serde::Serialize + 'static;
    fn delete_default_goal(&self, id: StatisticId) -> Result<()>;
}
