use anyhow::{Context, Result};
use rocket::form::Form;
use rocket::request::FlashMessage;
use rocket::response::{Flash, Redirect};
use rocket::serde::{Serialize};
use rocket_dyn_templates::{context, Template};
use backend::database::db_ops;
use backend::statistics::objects::{StatisticId, TimeFrameList};
use backend::statistics::statistic_repository::get_all_statistics;
use backend::statistics::type_definitions::{AnyStatistic, StatisticTypeSpecific};
use backend::util;
use backend::util::to_string::format_duration;
use backend::util::util::get_date_today;
use backend::statistics::type_definitions::Statistic;

#[derive(FromForm)]
pub struct GoalUpdateForm {
    statistic_id: StatisticId,
    goal_value: String,
}

#[derive(Debug, Serialize)]
struct StatisticGoalInfo {
    id: u32,
    name: String,
    category: String,
    current_goal: Option<String>,
    example_format: Option<String>,
}

impl StatisticGoalInfo {
    fn from_statistic(stat: &AnyStatistic) -> Result<Self> {
        let conn = db_ops::open_db()?;
        let today = get_date_today();

        let (current_goal, example_format) = match stat {
            AnyStatistic::IntegerAmount(s) => {
                let row = s.get_row_for_date(&conn, today)?;
                (row.goal.map(|g| g.to_string()), Some("Example: 42".to_string()))
            },
            AnyStatistic::TimeAmount(s) => {
                let row = s.get_row_for_date(&conn, today)?;
                (
                    row.goal.map(|d| format_duration(d)),
                    Some("Example: 2h, 30m, or 90m".to_string())
                )
            },
            AnyStatistic::TimeSpans(s) => {
                let row = s.get_row_for_date(&conn, today)?;
                (
                    row.goal.map(|tf| tf.to_string()),
                    Some("Example: 09:00-12:00, 14:00-17:00".to_string())
                )
            },
            AnyStatistic::PointInTime(s) => {
                let row = s.get_row_for_date(&conn, today)?;
                (
                    row.goal.map(|t| t.format("%H:%M:%S").to_string()),
                    Some("Example: 09:00:00".to_string())
                )
            },
            AnyStatistic::Habit(s) => {
                let row = s.get_row_for_date(&conn, today)?;
                (row.goal.map(|b| b.to_string()), Some("Example: true or false".to_string()))
            },
        };

        Ok(StatisticGoalInfo {
            id: stat.get_id().0,
            name: stat.get_name().to_string(),
            category: stat.get_category().to_string(),
            current_goal,
            example_format,
        })
    }
}

#[get("/goals")]
pub async fn show_goals(flash: Option<FlashMessage<'_>>) -> Template {
    let statistics = get_all_statistics()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|stat| StatisticGoalInfo::from_statistic(&stat).ok())
        .collect::<Vec<_>>();

    Template::render("edit_goals", context! {
        active_page: "goals",
        statistics: statistics,
        flash: flash.map(|msg| (msg.kind().to_string(), msg.message().to_string()))
    })
}

#[post("/goals", data = "<form>")]
pub async fn update_goal(form: Form<GoalUpdateForm>) -> Flash<Redirect> {
    match update_statistic_goal(form.into_inner()) {
        Ok(_) => Flash::success(
            Redirect::to(uri!(show_goals)),
            "Goal updated successfully"
        ),
        Err(e) => Flash::error(
            Redirect::to(uri!(show_goals)),
            format!("Failed to update goal: {}", e)
        )
    }
}

fn update_statistic_goal(form: GoalUpdateForm) -> Result<()> {
    let conn = db_ops::open_db()?;
    let stat = get_all_statistics()?
        .into_iter()
        .find(|s| s.get_id() == form.statistic_id)
        .context("Statistic not found")?;

    let today = get_date_today();

    match stat {
        AnyStatistic::IntegerAmount(s) => {
            let value = form.goal_value.parse()?;
            s.set_goal(&conn, today, Some(value))?;
        },
        AnyStatistic::TimeAmount(s) => {
            let value = util::string_parsers::parse_duration(&form.goal_value)?;
            s.set_goal(&conn, today, Some(value))?;
        },
        AnyStatistic::TimeSpans(s) => {
            // Assume comma-separated list of time frames
            let frames = form.goal_value
                .split(',')
                .map(str::trim)
                .map(util::string_parsers::parse_time_frame)
                .collect::<Result<Vec<_>, _>>()?;

            let time_frame_list = TimeFrameList::new_from_vec(frames);
            s.set_goal(&conn, today, Some(time_frame_list))?;
        },
        AnyStatistic::PointInTime(s) => {
            let value = util::string_parsers::parse_unknown_time_format(&form.goal_value)
                .context("Invalid time format")?;
            s.set_goal(&conn, today, Some(value))?;
        },
        AnyStatistic::Habit(s) => {
            let value = util::string_parsers::parse_bool(&form.goal_value)?;
            s.set_goal(&conn, today, Some(value))?;
        },
    }

    Ok(())
}