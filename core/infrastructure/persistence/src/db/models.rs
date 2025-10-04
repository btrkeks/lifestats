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

#[derive(Debug, Queryable, Selectable, Insertable, AsChangeset, Clone)]
#[diesel(table_name = crate::db::schema::plots)]
pub struct PlotRaw {
    pub id: i32,
    pub statistic_id: i32,
    pub title: String,
    pub width: i32,
    pub height: i32,
    pub plot_type: String,
}

impl PlotRaw {
    pub fn from_plot_spec(spec: &lifestats_domain::plot::specification::PlotSpecification) -> Self {
        Self {
            id: spec.id.0 as i32,
            statistic_id: spec.statistic_id.0 as i32,
            title: spec.title.clone(),
            width: 800,  // Default width
            height: 600, // Default height
            plot_type: match spec.plot_type {
                lifestats_domain::plot::specification::PlotType::BarChart => "bar_chart",
                lifestats_domain::plot::specification::PlotType::LineChart => "line_chart",
                lifestats_domain::plot::specification::PlotType::TimeFrames => "time_frames",
                lifestats_domain::plot::specification::PlotType::Scatter => "scatter",
                lifestats_domain::plot::specification::PlotType::CircularProgress => {
                    "circular_progress"
                }
            }
            .to_string(),
        }
    }

    pub fn into_plot_spec(
        self,
    ) -> Result<lifestats_domain::plot::specification::PlotSpecification> {
        use lifestats_domain::plot::specification::{PlotSpecification, PlotType};
        use lifestats_domain::shared::ids::{PlotId, StatisticId};

        let plot_type = match self.plot_type.as_str() {
            "bar_chart" => PlotType::BarChart,
            "line_chart" => PlotType::LineChart,
            "time_frames" => PlotType::TimeFrames,
            "scatter" => PlotType::Scatter,
            "circular_progress" => PlotType::CircularProgress,
            _ => return Err(anyhow::anyhow!("Unknown plot type: {}", self.plot_type)),
        };

        Ok(PlotSpecification::new(
            PlotId(self.id as u32),
            self.title,
            StatisticId(self.statistic_id as u32),
            plot_type,
        ))
    }
}
