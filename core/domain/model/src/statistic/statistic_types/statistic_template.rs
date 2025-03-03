use crate::repositories::goal_repository::GoalRepositoryTrait;
use crate::repositories::logging_data_repository::LoggingDataRepositoryTrait;
use crate::statistic::objects::goal::Goal;
use crate::statistic::objects::logging_data::DateValue;
use crate::statistic::objects::update_policy::UpdatePolicy;
use crate::statistic::statistic::{Statistic, StatisticBase};
use anyhow::Result;
use chrono::{Local, NaiveDate};
use serde::Serialize;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;
use transfer_objects::statistic_metadata::StatisticMetadata;

#[derive(Clone)]
pub struct StatisticTemplate<LogValue, RetrieveValue, UpdatePolicyType, GoalType>
where
    LogValue: Display + FromStr + Debug,
    UpdatePolicyType: UpdatePolicy<RetrieveValue, LogValue>,
    GoalType: Goal<RetrieveValue>,
{
    metadata: StatisticMetadata,
    logging_data_repository: Arc<dyn LoggingDataRepositoryTrait<StoredType = RetrieveValue>>,
    goal_repository: Arc<dyn GoalRepositoryTrait<GoalType = GoalType, StoredValue = RetrieveValue>>,
    _phantom1: PhantomData<LogValue>,         // TODO: Necessary?
    _phantom2: PhantomData<UpdatePolicyType>, // TODO: Necessary?
}

impl<LogValue, RetrieveValue, UpdatePolicyType, GoalType>
    StatisticTemplate<LogValue, RetrieveValue, UpdatePolicyType, GoalType>
where
    LogValue: Display + FromStr + Debug,
    UpdatePolicyType:
        crate::statistic::objects::update_policy::UpdatePolicy<RetrieveValue, LogValue>,
    GoalType: Goal<RetrieveValue>,
{
    pub fn new(
        metadata: StatisticMetadata,
        logging_data_repository: Arc<dyn LoggingDataRepositoryTrait<StoredType = RetrieveValue>>,
        goal_repository: Arc<
            dyn GoalRepositoryTrait<GoalType = GoalType, StoredValue = RetrieveValue>,
        >,
    ) -> Self {
        Self {
            metadata,
            logging_data_repository,
            goal_repository,
            _phantom1: PhantomData,
            _phantom2: PhantomData,
        }
    }
}

impl<LogValue, RetrieveValue, UpdatePolicyType, GoalType> StatisticBase
    for StatisticTemplate<LogValue, RetrieveValue, UpdatePolicyType, GoalType>
where
    LogValue: Display + FromStr + Debug + Serialize,
    UpdatePolicyType: UpdatePolicy<RetrieveValue, LogValue>,
    GoalType: Goal<RetrieveValue>,
    RetrieveValue: Clone + Serialize + serde::de::DeserializeOwned + FromStr,
{
    fn get_scores_for_closed_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<DateValue<Option<f64>>>> {
        let value_goals = self.get_values_with_goals(start_date, end_date)?;
        Ok(value_goals
            .into_iter()
            .map(|value_goals| {
                let (value, goal) = value_goals.value;
                DateValue {
                    date: value_goals.date,
                    value: goal.map(|goal| goal.calculate_score(value.as_ref())),
                }
            })
            .collect())
    }

    fn get_mean_score_for_closed_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Option<f64>> {
        // TODO: Don't count first day if not logged? Activate this via a parameter?
        let (sum, count) = self
            .get_values_with_goals(start_date, end_date)?
            .into_iter()
            .filter_map(|date_value| {
                if let Some(goal) = date_value.value.1 {
                    Some(goal.get_percentage_achieved(date_value.value.0.as_ref()))
                } else {
                    None
                }
            })
            .fold((0.0, 0), |(sum, count), value| (sum + value, count + 1));

        if count > 0 {
            Ok(Some(sum / count as f64))
        } else {
            Ok(None)
        }
    }

    fn get_streak(&self) -> Result<Option<u32>> {
        // TODO: How to treat no goals?
        //       I think this function should return None if the default goal is None
        const BATCH_SIZE: i64 = 20;
        let mut streak = 0;
        let mut end_date = Local::now().date_naive();
        let mut is_first = true;
        let mut no_goals_in_succession = 0;

        loop {
            let start_date = end_date - chrono::Duration::days(BATCH_SIZE);
            let data = self.get_values_with_goals(start_date, end_date)?;

            // TODO: Assumes that the data is sorted by date. Verify with a test!!
            for (value, goal) in data.into_iter().rev().map(|v| v.value) {
                if value.is_none() && is_first {
                    // Skip the first day if it's not logged yet. Don't increase streak
                    is_first = false;
                    continue;
                }

                if let Some(goal) = goal {
                    if !goal.is_achieved(value.as_ref()) {
                        return Ok(Some(streak));
                    }
                } else {
                    // TODO
                    // It's hard to detect whether there will be anymore logged values in the future
                    if no_goals_in_succession >= 10 {
                        return Ok(Some(streak));
                    }

                    no_goals_in_succession += 1;
                    continue;
                }

                no_goals_in_succession = 0;
                is_first = false;
                streak += 1;
            }

            // Move to the next batch of days
            end_date = start_date - chrono::Duration::days(1);
        }
    }
}

impl<LogValue, RetrieveValue, UpdatePolicyType, GoalType> Statistic
    for StatisticTemplate<LogValue, RetrieveValue, UpdatePolicyType, GoalType>
where
    LogValue: Display + FromStr + Debug + Serialize,
    UpdatePolicyType: UpdatePolicy<RetrieveValue, LogValue>,
    GoalType: Goal<RetrieveValue>,
    RetrieveValue: Clone + Serialize + serde::de::DeserializeOwned + FromStr,
{
    type LogValue = LogValue;
    type RetrieveValue = RetrieveValue;
    type UpdatePolicyType = UpdatePolicyType;
    type GoalType = GoalType;

    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }

    fn get_goal_repository(
        &self,
    ) -> &Arc<dyn GoalRepositoryTrait<GoalType = Self::GoalType, StoredValue = Self::RetrieveValue>>
    {
        &self.goal_repository
    }

    fn get_logging_repository(
        &self,
    ) -> &Arc<dyn LoggingDataRepositoryTrait<StoredType = Self::RetrieveValue>> {
        &self.logging_data_repository
    }
}
