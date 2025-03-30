use anyhow::Result;
use chrono::NaiveDate;
use diesel::prelude::*;
use domain_models::plot::any_plot::AnyPlot;
use domain_models::objects::goal::Goal;
use domain_models::objects::logging_data::{DateValue, StatisticData};
use transfer_objects::statistic_id::StatisticId;

#[derive(Debug, Queryable, Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::sqlite_database::schema::logging_data)]
pub struct LoggingDataRaw {
    pub statistic_id: i32,
    pub date: String,
    pub value: String,
}

impl LoggingDataRaw {
    pub fn into_value_logging_data<T>(self) -> DateValue<T>
    where
        T: serde::de::DeserializeOwned,
    {
        // TODO: Remove unwrap
        // println!("Parsing: {}", self.value);
        DateValue::new(
            NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").unwrap(),
            serde_json::from_str(&self.value)
                .map_err(|_| {
                    anyhow::anyhow!(
                        "Failed to parse value {} for statistic_id: {}",
                        self.value,
                        self.statistic_id
                    )
                })
                .unwrap(),
        )
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
    pub fn from_goal_logging_data<T, G: Goal<T>>(data: StatisticData<G>) -> Result<Self>
    where
        T: serde::Serialize,
    {
        // When data.value is None, we still want to create a GoalDataRaw
        // with explicit NULL values to indicate an intentionally unset goal
        let (target_value, extra_data1) = if let Some(value) = data.value {
            (
                Some(serde_json::to_string(&value.get_value())?),
                value.get_extra_data_1(),
            )
        } else {
            (None, None)
        };

        Ok(Self {
            statistic_id: data.statistic_id.0 as i32,
            date: data.date.to_string(),
            target_value,
            extra_data1,
        })
    }

    pub fn into_goal_date_value<T, G: Goal<T>>(self) -> DateValue<Option<G>>
    where
        T: serde::de::DeserializeOwned,
    {
        // TODO: Remove unwraps

        let goal = if let Some(value) = self.target_value {
            let mut goal = G::new_with_target(
                serde_json::from_str(&value)
                    .map_err(|_| {
                        anyhow::anyhow!(
                            "Failed to parse goal target value {} for statistic_id: {}",
                            value,
                            self.statistic_id
                        )
                    })
                    .unwrap(),
            );

            if let Some(extra_data_1) = self.extra_data1 {
                goal = goal.set_extra_data_1(&extra_data_1).unwrap();
            }

            Some(goal)
        } else {
            None
        };

        DateValue::new(
            NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").unwrap(),
            goal,
        )
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
    pub fn into_goal<T, G: Goal<T>>(self) -> Result<G>
    where
        T: serde::de::DeserializeOwned,
    {
        let target = serde_json::from_str(&self.target_value).map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse default goal target value {} for statistic_id {}: '{}'",
                self.target_value,
                self.statistic_id,
                e
            )
        })?;

        let mut goal = G::new_with_target(target);

        if let Some(extra_data_1) = self.extra_data1 {
            goal = goal.set_extra_data_1(&extra_data_1).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to set default goal extra_data_1 '{}' for statistic_id {}: '{}'",
                    extra_data_1,
                    self.statistic_id,
                    e
                )
            })?;
        }

        Ok(goal)
    }

    pub fn from_goal<T, G: Goal<T>>(statistic_id: StatisticId, goal: G) -> Self
    where
        T: serde::Serialize,
    {
        let target_value = serde_json::to_string(&goal.get_value()).unwrap();
        let extra_data1 = goal.get_extra_data_1();

        Self {
            statistic_id: statistic_id.0 as i32,
            target_value,
            extra_data1,
        }
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
