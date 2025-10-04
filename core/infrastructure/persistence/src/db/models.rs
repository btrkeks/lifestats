use anyhow::{Context, Result};
use chrono::NaiveDate;
use diesel::prelude::*;
use lifestats_domain::shared::dated_value::DatedValue;
use lifestats_domain::shared::ids::StatisticId;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Debug, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::db::schema::logging_data)]
pub struct LoggingDataRaw {
    pub statistic_id: i32,
    pub date: String,
    pub value: String,
}

impl LoggingDataRaw {
    pub fn into_dated_value<T>(self) -> Result<DatedValue<T>>
    where
        T: DeserializeOwned,
    {
        let date = NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").with_context(|| {
            format!(
                "Failed to parse log date '{}' for statistic_id {}",
                self.date, self.statistic_id
            )
        })?;

        let value = serde_json::from_str(&self.value).with_context(|| {
            format!(
                "Failed to parse log value '{}' for statistic_id {} on {}",
                self.value, self.statistic_id, date
            )
        })?;

        Ok(DatedValue::new(date, value))
    }
}

#[derive(Debug, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::db::schema::goals)]
pub struct GoalDataRaw {
    pub statistic_id: i32,
    pub date: String,
    #[diesel(treat_none_as_null = true)]
    pub goal: Option<String>,
}

impl GoalDataRaw {
    pub fn from_goal<G>(statistic_id: StatisticId, date: NaiveDate, goal: Option<G>) -> Result<Self>
    where
        G: Serialize,
    {
        let serialized_goal = match goal {
            Some(goal) => Some(serde_json::to_string(&goal).with_context(|| {
                format!(
                    "Failed to serialize goal for statistic_id {} on {}",
                    statistic_id.0, date
                )
            })?),
            None => None,
        };

        Ok(Self {
            statistic_id: statistic_id.0 as i32,
            date: date.to_string(),
            goal: serialized_goal,
        })
    }

    pub fn into_optional_goal<G>(self) -> Result<Option<G>>
    where
        G: DeserializeOwned,
    {
        match self.goal {
            Some(value) => {
                let goal = serde_json::from_str(&value).with_context(|| {
                    format!(
                        "Failed to parse goal '{}' for statistic_id {} on {}",
                        value, self.statistic_id, self.date
                    )
                })?;
                Ok(Some(goal))
            }
            None => Ok(None),
        }
    }

    pub fn into_dated_goal<G>(self) -> Result<DatedValue<Option<G>>>
    where
        G: DeserializeOwned,
    {
        let date = NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").with_context(|| {
            format!(
                "Failed to parse goal date '{}' for statistic_id {}",
                self.date, self.statistic_id
            )
        })?;

        let goal = self.into_optional_goal::<G>()?;
        Ok(DatedValue::new(date, goal))
    }
}

#[derive(Debug, Queryable, Selectable, Insertable, AsChangeset, Clone)]
#[diesel(table_name = crate::db::schema::default_goals)]
pub struct DefaultGoalRaw {
    pub statistic_id: i32,
    pub goal: String, // The single JSON blob
}
