// web_server/src/goal_edit/goal_edit_page.rs
use crate::guards::Context;
use chrono::NaiveDate;
use rocket::form::Form;
use rocket::http::Status;
use rocket::request::FlashMessage;
use rocket::response::status::Custom;
use rocket::response::{Flash, Redirect};
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use rocket_dyn_templates::{context, Template};
use transfer_objects::statistic_id::StatisticId;
use web_api::goal::{DeleteGoalRequest, GoalResponse, SetGoalRequest};

#[derive(Debug, FromForm)]
pub struct GoalForm {
    statistic_id: u32,
    target: String,
    extra_data: Option<String>,
    date: Option<String>, // Will be validated in handler
}

#[derive(Debug, Serialize)]
struct StatisticSelection {
    id: StatisticId,
    name: String,
    #[serde(rename = "type")]
    stat_type: String,
}

#[get("/goals")]
pub async fn show_goals(context: Context, flash: Option<FlashMessage<'_>>) -> Template {
    let statistics = lifestats_core::statistics::get_all_statistics(&context.0).unwrap();

    Template::render(
        "goals",
        context! {
            active_page: "goals",
            statistics: statistics.iter().map(|s| StatisticSelection {
                id: s.id(),
                name: s.get_name().to_string(),
                stat_type: s.get_type_name().to_string(),
            }).collect::<Vec<_>>(),
            flash: flash.map(|f| (f.kind().to_string(), f.message().to_string()))
        },
    )
}

#[post("/api/goals", format = "json", data = "<request>")]
pub async fn set_goal_api(
    context: Context,
    request: Json<SetGoalRequest>,
) -> Result<Json<GoalResponse>, Custom<Json<GoalResponse>>> {
    let date = request
        .date
        .as_ref()
        .map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d"))
        .transpose()
        .map_err(|_| {
            Custom(
                Status::BadRequest,
                Json(GoalResponse::error("Invalid date format".into())),
            )
        })?;

    let result = if let Some(date) = date {
        lifestats_core::goal_management::set_goal_for_date(
            &context.0,
            request.statistic_id,
            date,
            &request.target,
            request.extra_data.as_deref(),
        )
    } else {
        lifestats_core::goal_management::set_default_goal(
            &context.0,
            request.statistic_id,
            &request.target,
            request.extra_data.as_deref(),
        )
    };

    match result {
        Ok(_) => Ok(Json(GoalResponse::success("Goal set successfully".into()))),
        Err(e) => Err(Custom(
            Status::BadRequest,
            Json(GoalResponse::error(e.to_string())),
        )),
    }
}

#[delete("/api/goals", format = "json", data = "<request>")]
pub async fn delete_goal_api(
    context: Context,
    request: Json<DeleteGoalRequest>,
) -> Result<Json<GoalResponse>, Custom<Json<GoalResponse>>> {
    let date = request
        .date
        .as_ref()
        .map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d"))
        .transpose()
        .map_err(|_| {
            Custom(
                Status::BadRequest,
                Json(GoalResponse::error("Invalid date format".into())),
            )
        })?;

    let result = if let Some(date) = date {
        lifestats_core::goal_management::delete_goal_for_date(
            &context.0,
            request.statistic_id,
            date,
        )
    } else {
        lifestats_core::goal_management::delete_default_goal(&context.0, request.statistic_id)
    };

    match result {
        Ok(_) => Ok(Json(GoalResponse::success(
            "Goal deleted successfully".into(),
        ))),
        Err(e) => Err(Custom(
            Status::BadRequest,
            Json(GoalResponse::error(e.to_string())),
        )),
    }
}

#[get("/api/goals/current?<statisticId>&<date>")]
pub async fn get_current_goal(
    context: Context,
    statisticId: u32,
    date: Option<String>,
) -> Result<Json<Option<CurrentGoal>>, Custom<Json<GoalResponse>>> {
    let stat_id = StatisticId(statisticId);

    let date = date.and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());

    let goal = if let Some(date) = date {
        lifestats_core::goal_management::get_goal_for_date(&context.0, stat_id, date)
    } else {
        lifestats_core::goal_management::get_default_goal(&context.0, stat_id)
    }
    .map_err(|e| Custom(Status::BadRequest, Json(GoalResponse::error(e.to_string()))))?;

    Ok(Json(goal.map(|g| CurrentGoal {
        target: g.target,
        extra_data: g.extra_data_1,
    })))
}

#[derive(Serialize)]
struct CurrentGoal {
    target: String,
    extra_data: Option<String>,
}
