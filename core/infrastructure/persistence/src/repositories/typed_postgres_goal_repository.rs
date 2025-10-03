use std::sync::Arc;
use chrono::NaiveDate;
use lifestats_domain::statistic::repositories::goal_repository::GoalRepository;
use lifestats_domain::value_objects::dated_value::DatedValue;
use lifestats_domain::shared::ids::StatisticId;
use anyhow::Result;
use lifestats_domain::value_objects::date_range::DateRange;
use crate::repositories::postgres_goal_repository::PostgresGoalRepository;

pub struct TypedPostgresGoalRepository<T> {
    untyped_repository: Arc<PostgresGoalRepository>,
}

impl<T> GoalRepository for TypedPostgresGoalRepository<T> {
    type GoalType = T;

    fn get(&self, statistic_id: StatisticId, date: NaiveDate) -> Result<Option<Self::GoalType>> {
        let untyped_value = self.untyped_repository.get(statistic_id, date)?;

        untyped_value
            .map(|v| serde_json::from_value(v))
            .transpose()
            .into()
    }

    fn get_range(&self, statistic_id: StatisticId, date_range: DateRange) -> Result<Vec<DatedValue<Self::GoalType>>> {
        let untyped_range = self.untyped_repository.get_range(statistic_id, date_range)?;
        untyped_range
            .into_iter()
            .map(|dated_value| {
                let value = serde_json::from_value(dated_value.value)?;
                Ok(DatedValue {
                    date: dated_value.date,
                    value,
                })
            })
            .collect()
    }

    fn save(&self, statistic_id: StatisticId, date: NaiveDate, value: Self::GoalType) -> Result<()> {
        let serialized = serde_json::to_value(value)?;
        self.untyped_repository.save(
            statistic_id,
            date,
            serialized,
        )
    }
}