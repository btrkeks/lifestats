use chrono::NaiveDate;
use lifestats_domain::statistic::repositories::goal_repository::GoalRepository;
use lifestats_domain::value_objects::dated_value::DatedValue;
use lifestats_domain::shared::ids::StatisticId;
use anyhow::Result;
use lifestats_domain::value_objects::date_range::DateRange;

pub struct PostgresGoalRepository {
}

impl PostgresGoalRepository {
    pub(crate) fn get(&self, statistic_id: StatisticId, date: NaiveDate) -> Result<Option<serde_json::Value>> {
        todo!()
    }

    pub(crate) fn get_range(&self, statistic_id: StatisticId, date_range: DateRange) -> Result<Vec<DatedValue<serde_json::Value>>> {
        todo!()
    }

    pub(crate) fn save(&self, statistic_id: StatisticId, date: NaiveDate, value: serde_json::Value) -> Result<()> {
        todo!()
    }
}