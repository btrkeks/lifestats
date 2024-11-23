pub mod time_spans;
pub mod time_amount;
pub mod point_in_time;
pub mod integer_amount;
pub mod habit;

use std::fmt::Display;
use std::str::FromStr;
use crate::statistics::type_definitions::integer_amount::IntegerAmountStatistic;
use crate::database::db_ops;
use crate::statistics::objects::{StatisticId, StatisticRow};
use rusqlite::{Connection};
use crate::database::db_ops::DbStorable;
use crate::statistics::update_policy::{UpdatePolicy};
use anyhow::{anyhow, Context, Result};
use chrono::{NaiveDate};
use enum_dispatch::enum_dispatch;
use log::debug;
use serde::{Deserialize, Serialize};
use crate::statistics::goal::Goal;
use crate::statistics::type_definitions::habit::HabitStatistic;
use crate::statistics::type_definitions::point_in_time::PointInTimeStatistic;
use crate::statistics::type_definitions::time_amount::TimeAmountStatistic;
use crate::statistics::type_definitions::time_spans::TimeSpansStatistic;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StatisticMetadata {
    id: StatisticId,
    category: String,
    name: String,
}

#[enum_dispatch]
pub trait Statistic {
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
}

pub trait StatisticTypeSpecific: Statistic {
    type LogValue: DbStorable + FromStr + Display;
    type RetrieveValue: DbStorable + Clone;
    type UpdatePolicy: UpdatePolicy<
        Self::RetrieveValue,
        Self::LogValue,
        Output=Self::RetrieveValue,
    >;
    type GoalType: Goal<Self::RetrieveValue>;

    fn get_goal(&self) -> &Option<Self::GoalType>;

    // fn get_percentage_achieved(&self, value: &Self::RetrieveValue) -> Option<f64> {
    //     self.get_goal().map(|g| g.get_percentage_achieved(value))
    // }
    //
    // fn get_score(&self, value: &Self::RetrieveValue) -> Option<f64> {
    //     self.get_goal().map(|g| g.get_score(value))
    // }
    //
    // fn get_streak(&self, values: &[Option<Self::RetrieveValue>]) -> Option<u32> {
    //     self.get_goal().map(|g| g.get_streak(values))
    // }
    //
    // fn calculate_mean_score(&self, values: &[Option<Self::RetrieveValue>]) -> Option<f64> {
    //     self.get_goal().map(|g| g.calculate_mean_score(values))?
    // }

    fn set_goal(
        &self,
        conn: &Connection,
        date: NaiveDate,
        goal: Option<Self::RetrieveValue>,
    ) -> Result<()> {
        db_ops::set_goal(conn, self.get_id(), date, goal)
    }

    fn log_from_string(
        &self,
        conn: &Connection,
        date: NaiveDate,
        value_str: &str,
        update_policy_str: Option<&str>,
    ) -> Result<()> {
        let value = value_str.parse()
            .map_err(|_| anyhow!("Could not parse {}", value_str))?;

        let update_policy = update_policy_str
            .map_or(Ok(Self::UpdatePolicy::default()), |up| up.parse()
                .map_err(|_| anyhow!("Could not parse update policy {}", up))
            )?;

        self.log_value(conn, date, value, update_policy)
            .map_err(|_| {
                anyhow!(
                    "Logging a TimeSpans with id {} and TimeFrame {}",
                    self.get_id(),
                    value
                )
            })
    }

    fn log_value(
        &self,
        conn: &Connection,
        date: NaiveDate,
        new_value: Self::LogValue,
        update_policy: Self::UpdatePolicy,
    ) -> Result<()> {
        debug!("Logging value {} with update_policy {} for statistic {}", new_value, update_policy, self.get_name());
        // TODO: Use transactions / make read/write thread safe
        let mut row = self.get_row_for_date(conn, date)?;
        row.value = Some(update_policy.apply(row.value, new_value));
        db_ops::insert_row(conn, self.get_id(), &row)
    }

    fn get_row_for_date(
        &self,
        conn: &Connection,
        date: NaiveDate,
    ) -> Result<StatisticRow<Self::RetrieveValue>> {
        let row = db_ops::get_row_for_date(conn, self.get_id(), date)?;
        match row {
            Some(row) => Ok(row),
            // assume current goal if there is nothing logged
            // this means that the goal needs to be flushed to missing dates on change
            None => Ok(StatisticRow::new_without_value(date, self.get_goal())),
        }
    }

    // closed interval
    fn get_rows_for_range(
        &self,
        conn: &Connection,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<StatisticRow<Self::RetrieveValue>>> {
        db_ops::get_rows_for_range::<Self::RetrieveValue>(
            conn,
            self.get_id(),
            start,
            end,
            None,
            self.get_goal(),
        )
    }
}

// -----------------------------------------------

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[enum_dispatch(Statistic)]
pub enum AnyStatistic {
    IntegerAmount(IntegerAmountStatistic),
    TimeAmount(TimeAmountStatistic),
    PointInTime(PointInTimeStatistic),
    TimeSpans(TimeSpansStatistic),
    Habit(HabitStatistic),
}

impl AnyStatistic {
    fn log_from_string(
        &self,
        conn: &Connection,
        date: NaiveDate,
        value: &str,
        update_policy: Option<&str>,
    ) -> Result<()> {
        match self {
            AnyStatistic::IntegerAmount(s) => s.log_from_string(conn, date, value, update_policy),
            AnyStatistic::TimeAmount(s) => s.log_from_string(conn, date, value, update_policy),
            AnyStatistic::PointInTime(s) => s.log_from_string(conn, date, value, update_policy),
            AnyStatistic::TimeSpans(s) => s.log_from_string(conn, date, value, update_policy),
            AnyStatistic::Habit(s) => s.log_from_string(conn, date, value, update_policy),
        }
    }

    // pub fn get_score(&self, conn: &Connection, date: NaiveDate) -> Result<Option<f64>> {
    //     match self {
    //         AnyStatistic::IntegerAmount(s) => s.get_score(conn, date),
    //         AnyStatistic::TimeAmount(s) => s.get_score(conn, date),
    //         AnyStatistic::PointInTime(s) => s.get_score(conn, date),
    //         AnyStatistic::TimeSpans(s) => s.get_score(conn, date),
    //         AnyStatistic::Habit(s) => s.get_score(conn, date),
    //     }
    // }
}

// TODO: test that calculate_percent_in_relation_to_goal always returns 0 if there is no value
// TODO: test that get_rows_for_range includes the start end end date and has exactly (start-end+1) elements
// TODO: Add a test to ensure that fill_missing_dates_with_goal works as expected
// TODO: Tests for correct percentages