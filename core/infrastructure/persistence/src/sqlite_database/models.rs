use anyhow::{Context, Result};
use chrono::NaiveDate;
use diesel::prelude::*;
use lifestats_domain::plot::any_plot::AnyPlot;
use lifestats_domain::shared::dated_value::DatedValue;
use lifestats_domain::shared::ids::StatisticId;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Debug, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::sqlite_database::schema::logging_data)]
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
#[diesel(table_name = crate::sqlite_database::schema::goals)]
pub struct GoalDataRaw {
    pub statistic_id: i32,
    pub date: String,
    #[diesel(treat_none_as_null = true)]
    pub target_value: Option<String>,
    #[diesel(treat_none_as_null = true)]
    pub extra_data1: Option<String>,
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
            target_value: serialized_goal,
            extra_data1: None,
        })
    }

    pub fn into_optional_goal<G>(self) -> Result<Option<G>>
    where
        G: DeserializeOwned,
    {
        match self.target_value {
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

#[derive(Debug, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::sqlite_database::schema::default_goals)]
pub struct DefaultGoalRaw {
    pub statistic_id: i32,
    pub target_value: String,
    pub extra_data1: Option<String>,
}

impl DefaultGoalRaw {
    pub fn into_goal<G>(self) -> Result<G>
    where
        G: DeserializeOwned,
    {
        serde_json::from_str(&self.target_value).with_context(|| {
            format!(
                "Failed to parse default goal '{}' for statistic_id {}",
                self.target_value, self.statistic_id
            )
        })
    }

    pub fn start_date(&self) -> Result<Option<NaiveDate>> {
        self.extra_data1
            .as_deref()
            .map(|raw| {
                NaiveDate::parse_from_str(raw, "%Y-%m-%d").with_context(|| {
                    format!(
                        "Failed to parse default goal start date '{}' for statistic_id {}",
                        raw, self.statistic_id
                    )
                })
            })
            .transpose()
    }

    pub fn from_goal<G>(statistic_id: StatisticId, goal: G, start_date: NaiveDate) -> Result<Self>
    where
        G: Serialize,
    {
        let target_value = serde_json::to_string(&goal).with_context(|| {
            format!(
                "Failed to serialize default goal for statistic_id {}",
                statistic_id.0
            )
        })?;

        Ok(Self {
            statistic_id: statistic_id.0 as i32,
            target_value,
            extra_data1: Some(start_date.to_string()),
        })
    }
}

#[derive(Debug, Queryable, Selectable, Insertable, AsChangeset, Identifiable)]
#[diesel(table_name = crate::sqlite_database::schema::plots)]
#[diesel(primary_key(id))]
pub struct PlotRaw {
    pub id: i32,
    pub statistic_id: i32,
    pub title: String,
    pub width: i32,
    pub height: i32,
    pub plot_type: String,
}

impl TryFrom<&AnyPlot> for PlotRaw {
    type Error = anyhow::Error;

    fn try_from(plot: &AnyPlot) -> Result<Self, Self::Error> {
        let plot_type = match plot {
            AnyPlot::Bar(_) => "Bar".to_string(),
            AnyPlot::CircularProgressBar(_) => "CircularProgressBar".to_string(),
            AnyPlot::Scatter(_) => "Scatter".to_string(),
            AnyPlot::TimeFrames(_) => "TimeFrames".to_string(),
        };

        Ok(PlotRaw {
            id: plot.get_id().0 as i32,
            statistic_id: plot.get_statistic_id().0 as i32,
            width: plot.get_width() as i32,
            height: plot.get_height() as i32,
            title: plot.get_title().to_string(),
            plot_type,
        })
    }
}
