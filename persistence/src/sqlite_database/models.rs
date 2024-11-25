use diesel::prelude::*;
use chrono::NaiveDate;
use serde::{Serialize, Deserialize};
use crate::schema::logging_data;
use crate::statistics::objects::goal::Goal;
use crate::statistics::objects::statistic_metadata::StatisticId;

#[derive(Debug, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = logging_data)]
pub struct LoggingDataRaw {
    pub statistic_id: i32,
    pub date: String,
    pub value: Option<String>,
    pub goal: Option<String>,
}

// New generic wrapper for type-safe handling
#[derive(Debug)]
pub struct LoggingData<T, G = T>
where
    T: Serialize + for<'de> Deserialize<'de>,
    G: Goal<T> + Serialize + for<'de> Deserialize<'de>,
{
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub value: Option<T>,
    pub goal: Option<G>,
}

impl<T, G> LoggingData<T, G>
where
    T: Serialize + for<'de> Deserialize<'de>,
    G: Goal<T> + Serialize + for<'de> Deserialize<'de>,
{
    pub fn new(
        statistic_id: StatisticId,
        date: NaiveDate,
        value: Option<T>,
        goal: Option<G>,
    ) -> Self {
        Self {
            statistic_id,
            date,
            value,
            goal,
        }
    }

    // Convert to raw DB form
    pub fn into_raw(self) -> anyhow::Result<LoggingDataRaw> {
        Ok(LoggingDataRaw {
            statistic_id: self.statistic_id.0 as i32,
            date: self.date.to_string(),
            value: self.value.map(|v| serde_json::to_string(&v)).transpose()?,
            goal: self.goal.map(|g| serde_json::to_string(&g)).transpose()?,
        })
    }

    // Convert from raw DB form
    pub fn from_raw(raw: LoggingDataRaw) -> anyhow::Result<Self> {
        Ok(Self {
            statistic_id: StatisticId(raw.statistic_id as u32),
            date: NaiveDate::parse_from_str(&raw.date, "%Y-%m-%d")?,
            value: raw.value.map(|v| serde_json::from_str(&v)).transpose()?,
            goal: raw.goal.map(|g| serde_json::from_str(&g)).transpose()?,
        })
    }
}