use anyhow::{Context, Result};
use chrono::NaiveDate;
use diesel::prelude::*;
use lifestats_core::shared::dated_value::DatedValue;
use lifestats_core::shared::ids::StatisticId;
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

#[derive(Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::db::schema::ebook_sync_events)]
pub struct EbookSyncEventRow {
    pub event_id: String,
    pub source_name: String,
    pub metric: String,
    pub statistic_id: i32,
    pub date: String,
    pub delta: i64,
    pub book_id: Option<String>,
    pub book_title: String,
    pub confirmed_position_after: i64,
    pub created_at: String,
}

#[derive(Debug, Queryable, Selectable)]
#[allow(dead_code)]
#[diesel(table_name = crate::db::schema::integrations)]
pub struct IntegrationRow {
    pub id: i32,
    pub source_name: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at_epoch_seconds: Option<i64>,
    pub scopes: Option<String>,
    pub last_sync_at: Option<String>,
    pub canonical_tags: Option<String>,
}

#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = crate::db::schema::integrations)]
pub struct IntegrationUpsert {
    pub source_name: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expires_at_epoch_seconds: Option<i64>,
    pub scopes: Option<String>,
    pub last_sync_at: Option<String>,
    pub canonical_tags: Option<String>,
}

#[derive(Debug, Queryable, QueryableByName, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::db::schema::progress_items)]
pub struct ProgressItemRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub id: i32,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub category: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub unit_kind: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub unit_label: Option<String>,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub target_value: i64,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub tags: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub canonical_tags: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub created_at: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub updated_at: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::db::schema::progress_items)]
pub struct NewProgressItemRow {
    pub name: String,
    pub category: String,
    pub unit_kind: String,
    pub unit_label: Option<String>,
    pub target_value: i64,
    pub tags: String,
    pub canonical_tags: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Queryable, QueryableByName, Selectable, Insertable)]
#[diesel(table_name = crate::db::schema::progress_checkpoints)]
pub struct ProgressCheckpointRow {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub id: i32,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub progress_item_id: i32,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub recorded_at: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub effective_date: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub position_value: i64,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub source: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub source_event_id: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub note: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub created_at: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::db::schema::progress_checkpoints)]
pub struct NewProgressCheckpointRow {
    pub progress_item_id: i32,
    pub recorded_at: String,
    pub effective_date: String,
    pub position_value: i64,
    pub source: Option<String>,
    pub source_event_id: Option<String>,
    pub note: Option<String>,
    pub created_at: String,
}
