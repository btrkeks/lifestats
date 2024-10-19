use crate::db_ops;
use crate::objects::{StatisticRow, TimeFrame, TimeFrameList};
use rusqlite::Connection;
use std::str::FromStr;

use crate::db_ops::DbStorable;
use crate::statistic_getter::get_statistic_by_id;
use crate::string_parsers::{
    parse_bool, parse_duration, parse_time_frame, parse_unknown_time_format,
};
use crate::update_policy::{
    AmountUpdatePolicy, FixedValueUpdatePolicy, TimeFrameListUpdatePolicy, UpdatePolicy,
};
use anyhow::{anyhow, Context, Result};
use chrono::{Duration, NaiveDate, NaiveTime, Timelike};
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSeconds};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StatisticMetadata {
    id: u32,
    category: String,
    name: String,
}

#[enum_dispatch]
pub trait Statistic {
    fn get_metadata(&self) -> &StatisticMetadata;

    fn get_id(&self) -> u32 {
        self.get_metadata().id
    }

    fn get_category(&self) -> &str {
        &self.get_metadata().category
    }

    fn get_name(&self) -> &str {
        &self.get_metadata().name
    }

    fn log_from_string(
        &self,
        conn: &Connection,
        date: NaiveDate,
        value: &str,
        update_policy: Option<&str>,
    ) -> Result<()>;
}

pub trait StatisticTypeSpecific: Statistic {
    type LogValue: DbStorable;
    type RetrieveValue: DbStorable + Clone;
    type UpdatePolicy: UpdatePolicy<
        Self::RetrieveValue,
        Self::LogValue,
        Output = Self::RetrieveValue,
    >;

    fn get_goal(&self) -> Option<Self::RetrieveValue>;

    fn log_value(
        &self,
        conn: &Connection,
        date: NaiveDate,
        new_value: Self::LogValue,
        update_policy: Self::UpdatePolicy,
    ) -> Result<()> {
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

    // Returns 0 if the value wasn't set
    fn calculate_percent_in_relation_to_goal(
        &self,
        row: &StatisticRow<Self::RetrieveValue>,
    ) -> Option<f64>;

    fn get_percent_towards_goal(&self, conn: &Connection, date: NaiveDate) -> Result<Option<f64>> {
        let row = self.get_row_for_date(conn, date)?;
        Ok(self.calculate_percent_in_relation_to_goal(&row))
    }

    // Calculates the score for that row using its value and goal field
    // Returns None if row.goal is None
    fn calculate_score(&self, row: &StatisticRow<Self::RetrieveValue>) -> Option<f64> {
        self.calculate_percent_in_relation_to_goal(row)
            .map(|perc| perc.min(1.0))
    }

    fn get_score(&self, conn: &Connection, date: NaiveDate) -> Result<Option<f64>> {
        self.get_row_for_date(conn, date)
            .map(|row| self.calculate_score(&row))
    }

    // TODO: Assumes that rows with 0 values are included! Need to write tests for this
    fn calculate_mean_score(&self, rows: &[StatisticRow<Self::RetrieveValue>]) -> Option<f64> {
        let scores: Vec<f64> = rows
            .iter()
            .filter_map(|row| self.calculate_score(row))
            .collect();

        if scores.is_empty() {
            None
        } else {
            Some(scores.iter().sum::<f64>() / scores.len() as f64)
        }
    }

    fn parse_string_to_value(&self, string: &str) -> Result<Self::LogValue>;

    fn set_goal(
        &self,
        conn: &Connection,
        date: NaiveDate,
        goal: Option<Self::RetrieveValue>,
    ) -> Result<()> {
        db_ops::set_goal(conn, self.get_id(), date, goal)
    }

    fn set_goal_from_string(conn: &Connection, date: NaiveDate, value_str: Option<&str>) -> Result<()> {
        let value = if let Some(value_str) = value_str {
            self.parse_value_from_string(value_str)?
        }
        else {
            None
        }

        self.set_goal(conn, date, value)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IntegerAmountStatistic {
    metadata: StatisticMetadata,
    unit: String,
    goal: Option<i64>,
    is_good: bool,
}

impl Statistic for IntegerAmountStatistic {
    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }

    fn log_from_string(
        &self,
        conn: &Connection,
        date: NaiveDate,
        value_str: &str,
        update_policy: Option<&str>,
    ) -> Result<()> {
        let value: i64 = self.parse_string_to_value(value_str)?;

        let update_policy = if let Some(update_policy) = update_policy {
            AmountUpdatePolicy::from_str(update_policy).with_context(|| {
                anyhow!(
                    "Parsing update policy {} into an AmountUpdatePolicy for id {}.",
                    update_policy,
                    self.get_id()
                )
            })?
        } else {
            AmountUpdatePolicy::default()
        };

        self.log_value(conn, date, integer, update_policy)
            .with_context(|| anyhow!("Logging integer {} for id {}", self.get_id(), value))
    }
}

impl StatisticTypeSpecific for IntegerAmountStatistic {
    type LogValue = i64;
    type RetrieveValue = i64;
    type UpdatePolicy = AmountUpdatePolicy;

    fn get_goal(&self) -> Option<Self::RetrieveValue> {
        self.goal
    }

    fn calculate_percent_in_relation_to_goal(
        &self,
        row: &StatisticRow<Self::RetrieveValue>,
    ) -> Option<f64> {
        row.goal
            .map(|goal| row.value.unwrap_or(0) as f64 / goal as f64)
    }

    fn parse_string_to_value(&self, string: &str) -> Result<Self::LogValue> {
        value.parse().with_context(|| {
            anyhow!("Parsing {} into an integer for id {}", value, self.get_id())
        })?;
    }
}

#[serde_as]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeAmountStatistic {
    metadata: StatisticMetadata,
    #[serde_as(as = "Option<DurationSeconds<i64>>")]
    goal: Option<Duration>,
    is_good: bool,
}

impl Statistic for TimeAmountStatistic {
    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }

    fn log_from_string(
        &self,
        conn: &Connection,
        date: NaiveDate,
        value_str: &str,
        update_policy: Option<&str>,
    ) -> Result<()> {
        let value: i64 = self.parse_string_to_value(value_str)?;

        let update_policy = if let Some(update_policy) = update_policy {
            AmountUpdatePolicy::from_str(update_policy).with_context(|| {
                anyhow!(
                    "Parsing update policy {} into an AmountUpdatePolicy for id {}.",
                    update_policy,
                    self.get_id()
                )
            })?
        } else {
            AmountUpdatePolicy::default()
        };

        self.log_value(conn, date, duration, update_policy)
            .with_context(|| {
                anyhow!(
                    "Logging a TimeAmount of id {} with duration {}",
                    self.get_id(),
                    value,
                )
            })
    }
}

impl StatisticTypeSpecific for TimeAmountStatistic {
    type LogValue = Duration;
    type RetrieveValue = Duration;
    type UpdatePolicy = AmountUpdatePolicy;

    fn get_goal(&self) -> Option<Self::RetrieveValue> {
        self.goal
    }

    fn calculate_percent_in_relation_to_goal(
        &self,
        row: &StatisticRow<Self::RetrieveValue>,
    ) -> Option<f64> {
        row.goal.map(|goal| {
            if self.is_good {
                row.value.unwrap_or(Duration::zero()).num_seconds() as f64
                    / goal.num_seconds() as f64
            } else {
                goal.num_seconds() as f64
                    / row.value.unwrap_or(Duration::zero()).num_seconds() as f64
            }
        })
    }

    fn parse_string_to_value(&self, string: &str) -> Result<Self::LogValue> {
        parse_duration(value)
            .with_context(|| anyhow!("Parsing {} into a duration for id {}", value, self.get_id()))
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PointInTimeStatistic {
    metadata: StatisticMetadata,
    goal: Option<NaiveTime>,
}

impl Statistic for PointInTimeStatistic {
    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }

    fn log_from_string(
        &self,
        conn: &Connection,
        date: NaiveDate,
        value: &str,
        update_policy: Option<&str>,
    ) -> Result<()> {
        let time = parse_unknown_time_format(value).with_context(|| {
            anyhow!(
                "Parsing {} into a NaiveTime for id {}",
                value,
                self.get_id()
            )
        })?;
        let update_policy = if let Some(update_policy) = update_policy {
            FixedValueUpdatePolicy::from_str(update_policy).with_context(|| {
                anyhow!(
                    "Parsing update policy {} into a FixedValueUpdatePolicy for id {}.",
                    update_policy,
                    self.get_id()
                )
            })?
        } else {
            FixedValueUpdatePolicy::default()
        };

        self.log_value(conn, date, time, update_policy)
            .with_context(|| {
                anyhow!(
                    "Logging a PointInTime of id {} with time {}",
                    self.get_id(),
                    time
                )
            })
    }
}

impl StatisticTypeSpecific for PointInTimeStatistic {
    type LogValue = NaiveTime;
    type RetrieveValue = NaiveTime;
    type UpdatePolicy = FixedValueUpdatePolicy;

    fn get_goal(&self) -> Option<Self::RetrieveValue> {
        self.goal
    }

    fn calculate_percent_in_relation_to_goal(
        &self,
        row: &StatisticRow<Self::RetrieveValue>,
    ) -> Option<f64> {
        row.goal.map(|goal| {
            if let Some(value) = row.value {
                (1.0 + (goal.num_seconds_from_midnight() as f64
                    - value.num_seconds_from_midnight() as f64)
                    / (Duration::minutes(60).num_seconds() as f64))
                    .max(0.0)
            } else {
                0.0
            }
        })
    }
}

// -----------------------------------------------
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeSpansStatistic {
    metadata: StatisticMetadata,
    goal: Option<TimeFrameList>,
    #[serde(default = "default_goal_efficiency")]
    goal_efficiency: f64, // Number between 0 and 1 (percent)
}

fn default_goal_efficiency() -> f64 {
    1.0 // This represents 100%
}

impl Statistic for TimeSpansStatistic {
    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }

    fn log_from_string(
        &self,
        conn: &Connection,
        date: NaiveDate,
        value: &str,
        update_policy: Option<&str>,
    ) -> Result<()> {
        let value = parse_time_frame(value).with_context(|| {
            anyhow!(
                "Parsing {} into a TimeFrame for id {}",
                value,
                self.get_id()
            )
        })?;
        let update_policy = if let Some(update_policy) = update_policy {
            TimeFrameListUpdatePolicy::from_str(update_policy).with_context(|| {
                anyhow!(
                    "Parsing update policy {} into a TimeFrameListUpdatePolicy for id {}.",
                    update_policy,
                    self.get_id()
                )
            })?
        } else {
            TimeFrameListUpdatePolicy::default()
        };

        self.log_value(conn, date, value, update_policy)
            .with_context(|| {
                anyhow!(
                    "Logging a TimeSpans with id {} and TimeFrame {:?}",
                    self.get_id(),
                    value
                )
            })
    }
}

impl StatisticTypeSpecific for TimeSpansStatistic {
    type LogValue = TimeFrame;
    type RetrieveValue = TimeFrameList;
    type UpdatePolicy = TimeFrameListUpdatePolicy;

    fn get_goal(&self) -> Option<Self::RetrieveValue> {
        self.goal.clone()
    }

    fn calculate_percent_in_relation_to_goal(
        &self,
        row: &StatisticRow<Self::RetrieveValue>,
    ) -> Option<f64> {
        row.goal.as_ref().map(|goal| {
            if let Some(value) = &row.value {
                let goal = goal.get_total_duration().num_seconds() as f64 * self.goal_efficiency;
                let current_value = value.get_total_duration().num_seconds() as f64;

                current_value / goal
            } else {
                0.0
            }
        })
    }
}

// impl TimeSpansStatistic {
//     pub fn get_total_time_this_month(&self, conn: &Connection) -> Result<Duration> {
//         let (first_day, last_day) = get_current_month_range();
//         let rows = self.get_rows_for_range(conn, first_day, last_day)?;
//         let mut total_time: Duration = Duration::default();
//         for row in rows {
//             total_time += row.value.get_total_duration()
//         }
//         Ok(total_time)
//     }
// }

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HabitStatistic {
    metadata: StatisticMetadata,
    goal: Option<bool>,
}

impl Statistic for HabitStatistic {
    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }

    fn log_from_string(
        &self,
        conn: &Connection,
        date: NaiveDate,
        value: &str,
        update_policy: Option<&str>,
    ) -> Result<()> {
        let boolean = parse_bool(value)
            .with_context(|| anyhow!("Parsing {} into a bool for id {}", value, self.get_id()))?;
        let update_policy = if let Some(update_policy) = update_policy {
            FixedValueUpdatePolicy::from_str(update_policy).with_context(|| {
                anyhow!(
                    "Parsing update policy {} into a TimeFrameListUpdatePolicy for id {}.",
                    update_policy,
                    self.get_id()
                )
            })?
        } else {
            FixedValueUpdatePolicy::default()
        };

        self.log_value(conn, date, boolean, update_policy)
            .with_context(|| {
                anyhow!(
                    "Logging a Habit with id {} and value {}",
                    self.get_id(),
                    boolean
                )
            })
    }
}

impl StatisticTypeSpecific for HabitStatistic {
    type LogValue = bool;
    type RetrieveValue = bool;
    type UpdatePolicy = FixedValueUpdatePolicy;

    fn get_goal(&self) -> Option<Self::RetrieveValue> {
        self.goal
    }

    fn calculate_percent_in_relation_to_goal(
        &self,
        row: &StatisticRow<Self::RetrieveValue>,
    ) -> Option<f64> {
        row.goal.map(|goal| {
            if row.value.unwrap_or(false) == goal {
                1.0
            } else {
                0.0
            }
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeSpanStatistic {
    metadata: StatisticMetadata,
    start_stat_id: u32,
    end_stat_id: u32,
    goal: Option<TimeFrame>,
}

impl Statistic for TimeSpanStatistic {
    fn get_metadata(&self) -> &StatisticMetadata {
        &self.metadata
    }

    fn log_from_string(
        &self,
        _conn: &Connection,
        _date: NaiveDate,
        _value: &str,
        _update_policy: Option<&str>,
    ) -> Result<()> {
        Err(anyhow!("Cannot log for a TimeSpanStatistic")) // Refused Bequest code smell
    }
}

impl StatisticTypeSpecific for TimeSpanStatistic {
    type LogValue = TimeFrame;
    type RetrieveValue = TimeFrame;
    type UpdatePolicy = FixedValueUpdatePolicy;

    fn get_goal(&self) -> Option<Self::RetrieveValue> {
        self.goal
    }

    fn log_value(
        &self,
        _conn: &Connection,
        _date: NaiveDate,
        _value: Self::LogValue,
        _update_policy: Self::UpdatePolicy,
    ) -> Result<()> {
        unimplemented!()
    }

    fn get_row_for_date(
        &self,
        conn: &Connection,
        date: NaiveDate,
    ) -> Result<StatisticRow<Self::RetrieveValue>> {
        let start_stat = get_statistic_by_id(self.start_stat_id)?.unwrap();
        let end_stat = get_statistic_by_id(self.end_stat_id)?.unwrap();

        if let (AnyStatistic::PointInTime(start_stat), AnyStatistic::PointInTime(end_stat)) =
            (start_stat, end_stat)
        {
            let row_start = start_stat.get_row_for_date(conn, date)?;
            let row_end = end_stat.get_row_for_date(conn, date)?;

            let start_time = row_start.value.unwrap_or_default();
            let end_time = row_end.value.unwrap_or(start_time);
            let time_frame = TimeFrame::new(start_time, end_time)?;

            Ok(StatisticRow::new_with_value(date, time_frame, self.goal))
        } else {
            Err(anyhow!(
                "Any of the statistics is not a PointInTime statistic!"
            ))
        }
    }

    fn get_rows_for_range(
        &self,
        conn: &Connection,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<StatisticRow<Self::RetrieveValue>>> {
        let start_stat = get_statistic_by_id(self.start_stat_id)?.unwrap();
        let end_stat = get_statistic_by_id(self.end_stat_id)?.unwrap();

        if let (AnyStatistic::PointInTime(start_stat), AnyStatistic::PointInTime(end_stat)) =
            (start_stat, end_stat)
        {
            let mut result = Vec::new();
            let mut current_date = start;

            while current_date <= end {
                let row_start = start_stat.get_row_for_date(conn, current_date)?;
                let row_end = end_stat.get_row_for_date(conn, current_date)?;

                if let Some(start_time) = row_start.value {
                    let end_time = row_end.value.unwrap_or(start_time);

                    match TimeFrame::new(start_time, end_time) {
                        Ok(time_frame) => result.push(StatisticRow::new_with_value(
                            current_date,
                            time_frame,
                            self.goal,
                        )),
                        Err(_) => {
                            // If the TimeFrame creation fails, we'll skip this day but log a warning
                            log::warn!("Failed to create TimeFrame for date: {}", current_date);
                        }
                    }
                }

                current_date = current_date
                    .succ_opt()
                    .ok_or_else(|| anyhow!("Date overflow"))?;
            }

            Ok(result)
        } else {
            Err(anyhow!(
                "Any of the statistics is not a PointInTime statistic!"
            ))
        }
    }

    fn calculate_percent_in_relation_to_goal(
        &self,
        _row: &StatisticRow<Self::RetrieveValue>,
    ) -> Option<f64> {
        None // TODO: implement
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[enum_dispatch(Statistic)]
pub enum AnyStatistic {
    IntegerAmount(IntegerAmountStatistic),
    TimeAmount(TimeAmountStatistic),
    PointInTime(PointInTimeStatistic),
    TimeSpans(TimeSpansStatistic),
    TimeSpan(TimeSpanStatistic),
    Habit(HabitStatistic),
}

impl AnyStatistic {
    pub fn get_score(&self, conn: &Connection, date: NaiveDate) -> Result<Option<f64>> {
        match self {
            AnyStatistic::IntegerAmount(s) => s.get_score(conn, date),
            AnyStatistic::TimeAmount(s) => s.get_score(conn, date),
            AnyStatistic::PointInTime(s) => s.get_score(conn, date),
            AnyStatistic::TimeSpans(s) => s.get_score(conn, date),
            AnyStatistic::TimeSpan(s) => s.get_score(conn, date),
            AnyStatistic::Habit(s) => s.get_score(conn, date),
        }
    }
}

// TODO: test that calculate_percent_in_relation_to_goal always returns 0 if there is no value
// TODO: test that get_rows_for_range includes the start end end date and has exactly (start-end+1) elements
// TODO: Add a test to ensure that fill_missing_dates_with_goal works as expected
