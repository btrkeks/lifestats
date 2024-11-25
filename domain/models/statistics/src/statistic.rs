use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::str::FromStr;
use crate::type_definitions::integer_amount::IntegerAmountStatistic;
use anyhow::{anyhow, Result};
use chrono::{NaiveDate};
use log::{debug, Metadata};
use serde::{Deserialize, Serialize};
use crate::objects::goal::Goal;
use crate::objects::statistic_metadata::{StatisticId, StatisticMetadata};
use crate::objects::update_policies::UpdatePolicy;
use crate::type_definitions::habit::HabitStatistic;
use crate::type_definitions::point_in_time::PointInTimeStatistic;
use crate::type_definitions::time_amount::TimeAmountStatistic;
use crate::type_definitions::time_spans::TimeSpansStatistic;

pub trait Statistic {
    /// The type of data that can be logged
    /// E.g.: A time frame [10:00 - 11:30]
    type LogValue: Display + FromStr + Debug; // FromStr for log_from_string

    /// The type of data that will be stored in the database
    /// E.g.: A list of time frames [[10:00 - 11:30], [12:00 - 13:30]]
    type RetrieveValue;

    type UpdatePolicy: UpdatePolicy<Self::RetrieveValue, Self::LogValue>;

    type GoalType: Goal<Self::RetrieveValue> + serde::Serialize + for<'de> serde::Deserialize<'de>;

    fn get_metadata(&self) -> &StatisticMetadata;

    fn get_id(&self) -> StatisticId {
        self.get_metadata().id
    }

    fn get_category(&self) -> &str {
        &self.get_metadata().category
    }

    fn get_name(&self) -> &str {
        &self.get_metadata().name
    }

    fn get_goal(&self) -> &Option<Self::GoalType>;

    fn change_goal(&mut self, new_goal: Option<Self::GoalType>);

    fn get_percentage_achieved(&self, value: &Self::RetrieveValue) -> Option<f64> {
        self.get_goal().as_ref().map(|g| g.get_percentage_achieved(value))
    }

    fn get_score(&self, value: &Self::RetrieveValue) -> Option<f64> {
        self.get_goal().as_ref().map(|g| g.calculate_score(value))
    }

    fn calculate_mean_score(&self, values: &[Option<Self::RetrieveValue>]) -> Option<f64> {
        self.get_goal().as_ref().map(|g| g.calculate_mean_score(values))?
    }

    fn get_streak(&self, values: &[Option<Self::RetrieveValue>]) -> Option<u32> {
        self.get_goal().as_ref().map(|g| g.get_streak(values))
    }

    // TODO TODO TODO
    // fn log_from_string(
    //     &self,
    //     conn: &mut SqliteConnection,
    //     date: NaiveDate,
    //     value_str: &str,
    //     update_policy_str: Option<&str>,
    // ) -> Result<()> {
    //     let value = value_str.parse()
    //         .map_err(|_| anyhow!("Could not parse {}", value_str))?;
    //
    //     let update_policy = update_policy_str
    //         .map_or(Ok(Self::UpdatePolicy::default()), |up| up.parse()
    //             .map_err(|_| anyhow!("Could not parse update policy {}", up)),
    //         )?;
    //
    //     self.log_value(conn, date, value, update_policy)
    // }

    // fn log_value(
    //     &self,
    //     conn: &mut SqliteConnection,
    //     date: NaiveDate,
    //     new_value: Self::LogValue,
    //     update_policy: Self::UpdatePolicy,
    // ) -> Result<()> {
    //     debug!("Logging value {} with update_policy {} for statistic {}", new_value, update_policy, self.get_name());
    //
    //     conn.transaction(|conn| {
    //         let mut row = self.get_row_for_date(conn, date)?;
    //         row.value = Some(update_policy.apply(row.value, new_value));
    //         StatisticQueries::insert_or_update_row(conn, row)
    //     })
    // }
    //
    // fn get_row_for_date(
    //     &self,
    //     conn: &mut SqliteConnection,
    //     date: NaiveDate,
    // ) -> Result<LoggingData<Self::RetrieveValue, Self::GoalType>> {
    //     Ok(StatisticQueries::get_row(conn, self.get_id(), date)?
    //         .unwrap_or_else(|| LoggingData::new(
    //             self.get_id(),
    //             date,
    //             None,
    //             self.get_goal().clone(),
    //         )))
    // }
    //
    // fn get_rows_for_closed_range(
    //     &self,
    //     conn: &mut SqliteConnection,
    //     start: NaiveDate,
    //     end: NaiveDate,
    // ) -> Result<Vec<LoggingData<Self::RetrieveValue, Self::GoalType>>> {
    //     // TODO: Doesn't fill in empty rows. This might break other code
    //     StatisticQueries::get_rows_in_range(conn, self.get_id(), start, end)
    // }
}

// -----------------------------------------------

#[derive(Serialize, Deserialize)]
pub struct StatisticDefinition<LogValue, RetrieveValue, UpdatePolicy, GoalType> {
    metadata: StatisticMetadata,
    goal: Option<GoalType>,
    _marker: PhantomData<(LogValue, RetrieveValue, UpdatePolicy)>,
}

impl<LogValue, RetrieveValue, Policy, GoalType> StatisticDefinition<LogValue, RetrieveValue, Policy, GoalType> {
    pub fn new(metadata: StatisticMetadata, goal: Option<GoalType>) -> Self {
        Self {
            metadata,
            goal,
            _marker: PhantomData,
        }
    }
}

impl<LogValue, RetrieveValue, Policy, GoalType> Statistic
for StatisticDefinition<LogValue, RetrieveValue, Policy, GoalType>
where
    LogValue: Display + FromStr + Debug,
    Policy: UpdatePolicy<RetrieveValue, LogValue>,
    GoalType: Goal<RetrieveValue> + Serialize + for<'de> Deserialize<'de>,
{
    type LogValue = LogValue;
    type RetrieveValue = RetrieveValue;
    type UpdatePolicy = Policy;
    type GoalType = GoalType;

    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }

    fn get_goal(&self) -> &Option<Self::GoalType> {
        &self.goal
    }

    fn change_goal(&mut self, new_goal: Option<Self::GoalType>) {
        self.goal = new_goal;
    }
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum AnyStatistic {
    IntegerAmount(IntegerAmountStatistic),
    TimeAmount(TimeAmountStatistic),
    PointInTime(PointInTimeStatistic),
    TimeSpans(TimeSpansStatistic),
    Habit(HabitStatistic),
}

// TODO TODO TODO
// impl AnyStatistic {
//     fn log_from_string(
//         &self,
//         conn: &mut SqliteConnection,
//         date: NaiveDate,
//         value: &str,
//         update_policy: Option<&str>,
//     ) -> Result<()> {
//         match self {
//             AnyStatistic::IntegerAmount(s) => s.log_from_string(conn, date, value, update_policy),
//             AnyStatistic::TimeAmount(s) => s.log_from_string(conn, date, value, update_policy),
//             AnyStatistic::PointInTime(s) => s.log_from_string(conn, date, value, update_policy),
//             AnyStatistic::TimeSpans(s) => s.log_from_string(conn, date, value, update_policy),
//             AnyStatistic::Habit(s) => s.log_from_string(conn, date, value, update_policy),
//         }
//     }
//
//     pub fn get_score(&self, conn: &mut SqliteConnection, date: NaiveDate) -> Result<Option<f64>> {
//         match self {
//             AnyStatistic::IntegerAmount(s) => s.get_score(conn, date),
//             AnyStatistic::TimeAmount(s) => s.get_score(conn, date),
//             AnyStatistic::PointInTime(s) => s.get_score(conn, date),
//             AnyStatistic::TimeSpans(s) => s.get_score(conn, date),
//             AnyStatistic::Habit(s) => s.get_score(conn, date),
//         }
//     }
// }