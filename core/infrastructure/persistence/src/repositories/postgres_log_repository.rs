use chrono::NaiveDate;
use serde::de::DeserializeOwned;
use serde::Serialize;
use lifestats_domain::statistic::ports::log_repository::StatisticLogRepository;
use lifestats_domain::value_objects::date_range::DateRange;
use lifestats_domain::value_objects::dated_value::DatedValue;
use lifestats_domain::shared::ids::StatisticId;
use anyhow::Result;

pub struct PostgresLogRepository {}

impl PostgresLogRepository {
    pub fn get(&self, statistic_id: StatisticId, date: NaiveDate) -> Result<Option<serde_json::Value>> {
        todo!()
    }

    pub fn get_range(&self, statistic_id: StatisticId, date_range: DateRange) -> Result<Vec<DatedValue<serde_json::Value>>> {
        todo!()
    }

    pub fn save(&self, statistic_id: StatisticId, date: NaiveDate, value: serde_json::Value) -> Result<()> {
        todo!()
    }
}