use diesel::backend::Backend;
use diesel::deserialize::FromSql;
use diesel::prelude::*;
use diesel::serialize::{IsNull, Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;
use diesel::{deserialize, serialize, AsExpression, FromSqlRow};
use lifestats_domain::shared::ids::StatisticId;
use lifestats_domain::statistic::ports::goal_repositories::GoalRepositories;
use lifestats_domain::statistic::ports::logging_repositories::LoggingRepositories;
use lifestats_domain::statistic::statistic_types::any_statistic::{AnyStatistic, StatisticType};
use lifestats_domain::statistic::statistic_types::habit::HabitStatistic;
use lifestats_domain::statistic::statistic_types::integer_amount::IntegerAmountStatistic;
use lifestats_domain::statistic::statistic_types::point_in_time::PointInTimeStatistic;
use lifestats_domain::statistic::statistic_types::time_amount::TimeAmountStatistic;
use lifestats_domain::statistic::statistic_types::time_spans::TimeSpansStatistic;
use lifestats_domain::statistic::value_objects::statistic_metadata::StatisticMetadata;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
pub enum StatisticTypeDto {
    IntegerAmount,
    TimeAmount,
    PointInTime,
    TimeSpans,
    Habit,
}

impl ToSql<Text, Sqlite> for StatisticTypeDto {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        let value = match self {
            StatisticTypeDto::IntegerAmount => "IntegerAmount",
            StatisticTypeDto::TimeAmount => "TimeAmount",
            StatisticTypeDto::PointInTime => "PointInTime",
            StatisticTypeDto::TimeSpans => "TimeSpans",
            StatisticTypeDto::Habit => "Habit",
        };
        out.set_value(value);
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Sqlite> for StatisticTypeDto {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let str_value = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        match str_value.as_str() {
            "IntegerAmount" => Ok(Self::IntegerAmount),
            "TimeAmount" => Ok(Self::TimeAmount),
            "PointInTime" => Ok(Self::PointInTime),
            "TimeSpans" => Ok(Self::TimeSpans),
            "Habit" => Ok(Self::Habit),
            _ => Err("Invalid statistic type".into()),
        }
    }
}

impl From<StatisticType> for StatisticTypeDto {
    fn from(statistic_type: StatisticType) -> Self {
        match statistic_type {
            StatisticType::IntegerAmount => Self::IntegerAmount,
            StatisticType::TimeAmount => Self::TimeAmount,
            StatisticType::PointInTime => Self::PointInTime,
            StatisticType::TimeSpans => Self::TimeSpans,
            StatisticType::Habit => Self::Habit,
        }
    }
}

impl From<StatisticTypeDto> for StatisticType {
    fn from(value: StatisticTypeDto) -> Self {
        match value {
            StatisticTypeDto::IntegerAmount => Self::IntegerAmount,
            StatisticTypeDto::TimeAmount => Self::TimeAmount,
            StatisticTypeDto::PointInTime => Self::PointInTime,
            StatisticTypeDto::TimeSpans => Self::TimeSpans,
            StatisticTypeDto::Habit => Self::Habit,
        }
    }
}

#[derive(Debug, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::sqlite_database::schema::statistics)]
pub struct StatisticDto {
    pub id: i32,
    pub name: String,
    pub category: String,
    pub statistic_type: StatisticTypeDto,
}

impl StatisticDto {
    pub fn new(
        stat_id: StatisticId,
        name: String,
        category: String,
        statistic_type: StatisticType,
    ) -> Self {
        Self {
            id: stat_id.0 as i32,
            name,
            category,
            statistic_type: statistic_type.into(),
        }
    }

    pub fn from_any_statistic(statistic: &AnyStatistic) -> Self {
        Self {
            id: statistic.get_metadata().id().0 as i32,
            name: statistic.get_metadata().name().to_string(),
            category: statistic.get_metadata().category().to_string(),
            statistic_type: statistic.get_type().into(),
        }
    }

    pub fn into_any_statistic(
        self,
        logging_repositories: &LoggingRepositories,
        goal_repositories: &GoalRepositories,
    ) -> AnyStatistic {
        let metadata = StatisticMetadata::new(
            StatisticId(self.id as u32),
            self.category.into_boxed_str(),
            self.name.into_boxed_str(),
        );

        match self.statistic_type {
            StatisticTypeDto::IntegerAmount => {
                AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
                    metadata,
                    Arc::clone(&logging_repositories.integer_amount_repo),
                    Arc::clone(&goal_repositories.integer_amount_repo),
                ))
            }
            StatisticTypeDto::TimeAmount => AnyStatistic::TimeAmount(TimeAmountStatistic::new(
                metadata,
                Arc::clone(&logging_repositories.time_amount_repo),
                Arc::clone(&goal_repositories.time_amount_repo),
            )),
            StatisticTypeDto::TimeSpans => AnyStatistic::TimeSpans(TimeSpansStatistic::new(
                metadata,
                Arc::clone(&logging_repositories.time_spans_repo),
                Arc::clone(&goal_repositories.time_spans_repo),
            )),
            StatisticTypeDto::PointInTime => AnyStatistic::PointInTime(PointInTimeStatistic::new(
                metadata,
                Arc::clone(&logging_repositories.point_in_time_repo),
                Arc::clone(&goal_repositories.point_in_time_repo),
            )),
            StatisticTypeDto::Habit => AnyStatistic::Habit(HabitStatistic::new(
                metadata,
                Arc::clone(&logging_repositories.habit_repo),
                Arc::clone(&goal_repositories.habit_repo),
            )),
        }
    }
}
