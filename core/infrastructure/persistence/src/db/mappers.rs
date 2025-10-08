use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use diesel::backend::Backend;
use diesel::deserialize::FromSql;
use diesel::prelude::*;
use diesel::serialize::{IsNull, Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;
use diesel::{deserialize, serialize, AsExpression, FromSqlRow};
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::traits::base_statistic::Statistic;
use lifestats_core::statistic::types::logable_statistics::habit::HabitStatistic;
use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
use lifestats_core::statistic::types::logable_statistics::point_in_time::PointInTimeStatistic;
use lifestats_core::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;
use lifestats_core::statistic::value_objects::goals::amount_goal::AmountGoal;
use lifestats_core::statistic::value_objects::goals::finished_in_time_goal::FinishedInTimeGoal;
use lifestats_core::statistic::value_objects::goals::habit_goal::HabitGoal;
use lifestats_core::statistic::value_objects::goals::time_spans_goal::TimeSpansGoal;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::db::models::DefaultGoalRaw;

#[derive(Debug, Serialize, Deserialize, AsExpression, FromSqlRow, Clone, Copy)]
#[diesel(sql_type = Text)]
pub enum StatisticTypeDto {
    IntegerAmount,
    PointInTime,
    TimeSpans,
    Habit,
}

impl StatisticTypeDto {
    pub fn as_str(&self) -> &'static str {
        match self {
            StatisticTypeDto::IntegerAmount => "IntegerAmount",
            StatisticTypeDto::PointInTime => "PointInTime",
            StatisticTypeDto::TimeSpans => "TimeSpans",
            StatisticTypeDto::Habit => "Habit",
        }
    }
}

impl ToSql<Text, Sqlite> for StatisticTypeDto {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(self.as_str());
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for StatisticTypeDto {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let str_value = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        match str_value.as_str() {
            "IntegerAmount" => Ok(Self::IntegerAmount),
            "PointInTime" => Ok(Self::PointInTime),
            "TimeSpans" => Ok(Self::TimeSpans),
            "Habit" => Ok(Self::Habit),
            other => Err(anyhow!("Invalid statistic type '{other}'").into()),
        }
    }
}

#[derive(Debug, Queryable, QueryableByName, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::db::schema::statistics)]
pub struct StatisticDto {
    #[diesel(sql_type = diesel::sql_types::Integer)]
    pub id: i32,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub category: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub statistic_type: StatisticTypeDto,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub default_goal_valid_since: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub tags: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub canonical_tags: Option<String>,
}

impl StatisticDto {
    pub fn from_any_statistic(statistic: &AnyStatistic) -> Result<Self> {
        let (statistic_type, metadata) = match statistic {
            AnyStatistic::Habit(inner) => (StatisticTypeDto::Habit, inner.get_metadata()),
            AnyStatistic::IntegerAmount(inner) => {
                (StatisticTypeDto::IntegerAmount, inner.get_metadata())
            }
            AnyStatistic::PointInTime(inner) => {
                (StatisticTypeDto::PointInTime, inner.get_metadata())
            }
            AnyStatistic::TimeSpans(inner) => (StatisticTypeDto::TimeSpans, inner.get_metadata()),
            _ => {
                return Err(anyhow!(
                    "SQLite persistence does not support this statistic variant yet"
                ));
            }
        };

        // Serialize tags to JSON
        let tags_json =
            serde_json::to_string(metadata.tags()).context("failed to serialize tags to JSON")?;

        let canonical_tags = canonicalize_tags(metadata.tags());

        Ok(Self {
            id: metadata.id().0 as i32,
            name: metadata.name().to_string(),
            category: metadata.category().to_string(),
            statistic_type,
            default_goal_valid_since: statistic.get_default_goal_valid_since().to_string(),
            tags: tags_json,
            canonical_tags,
        })
    }

    pub fn into_any_statistic(self, default_goal: Option<DefaultGoalRaw>) -> Result<AnyStatistic> {
        // Deserialize tags from JSON
        let tags: std::collections::HashMap<String, String> =
            serde_json::from_str(&self.tags).context("failed to deserialize tags from JSON")?;

        // Create metadata and add tags
        let mut metadata =
            StatisticMetadata::new(StatisticId(self.id as u32), self.category, self.name).with_tag(
                "statistic_type".to_string(),
                self.statistic_type.as_str().to_string(),
            );

        // Apply all tags using the builder pattern
        for (key, value) in tags {
            metadata = metadata.with_tag(key, value);
        }

        // Parse the date from our new, reliable column.
        let start_date = NaiveDate::parse_from_str(&self.default_goal_valid_since, "%Y-%m-%d")
            .unwrap_or_else(|_| default_goal_start_fallback());

        match self.statistic_type {
            StatisticTypeDto::Habit => {
                let (goal, _) = parse_default_goal::<HabitGoal>(default_goal, start_date)?;
                Ok(AnyStatistic::Habit(HabitStatistic::new(
                    metadata, goal, start_date,
                )))
            }
            StatisticTypeDto::IntegerAmount => {
                let (goal, _) = parse_default_goal::<AmountGoal<i64>>(default_goal, start_date)?;
                Ok(AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
                    metadata, goal, start_date,
                )))
            }
            StatisticTypeDto::PointInTime => {
                let (goal, _) = parse_default_goal::<FinishedInTimeGoal>(default_goal, start_date)?;
                Ok(AnyStatistic::PointInTime(PointInTimeStatistic::new(
                    metadata, goal, start_date,
                )))
            }
            StatisticTypeDto::TimeSpans => {
                let (goal, _) = parse_default_goal::<TimeSpansGoal>(default_goal, start_date)?;
                Ok(AnyStatistic::TimeSpans(TimeSpansStatistic::new(
                    metadata, goal, start_date,
                )))
            }
        }
    }
}

#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = crate::db::schema::statistics)]
pub struct InsertStatisticDto {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub name: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub category: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub statistic_type: StatisticTypeDto,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub default_goal_valid_since: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub tags: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    pub canonical_tags: Option<String>,
}

impl InsertStatisticDto {
    pub fn from_any_statistic(statistic: &AnyStatistic) -> Result<Self> {
        let dto = StatisticDto::from_any_statistic(statistic)?;
        Ok(Self {
            name: dto.name,
            category: dto.category,
            statistic_type: dto.statistic_type,
            default_goal_valid_since: dto.default_goal_valid_since,
            tags: dto.tags,
            canonical_tags: dto.canonical_tags,
        })
    }
}

fn canonicalize_tags(tags: &HashMap<String, String>) -> Option<String> {
    if tags.is_empty() {
        return None;
    }

    let mut normalized: Vec<(String, String)> = tags
        .iter()
        .map(|(key, value)| (key.trim().to_ascii_lowercase(), value.trim().to_string()))
        .collect();

    normalized.sort_by(|a, b| a.0.cmp(&b.0));

    let fingerprint = normalized
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(";");

    if fingerprint.is_empty() {
        None
    } else {
        Some(fingerprint)
    }
}

fn parse_default_goal<G>(
    default_goal_raw: Option<DefaultGoalRaw>,
    start_date: NaiveDate,
) -> Result<(Option<G>, NaiveDate)>
where
    G: serde::de::DeserializeOwned,
{
    if let Some(raw) = default_goal_raw {
        let goal = serde_json::from_str(&raw.goal).with_context(|| {
            format!(
                "Failed to parse default goal '{}' for statistic_id {}",
                raw.goal, raw.statistic_id
            )
        })?;
        Ok((Some(goal), start_date))
    } else {
        Ok((None, start_date))
    }
}

fn default_goal_start_fallback() -> NaiveDate {
    NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid fallback date")
}
