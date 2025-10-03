use crate::guards::Context;
use crate::logger::log_core::log_statistic;
use rocket::form::Form;
use rocket::get;
use rocket::post;
use rocket::request::FlashMessage;
use rocket::response::{Flash, Redirect};
use rocket::serde::Serialize;
use rocket_dyn_templates::{context, Template};
use transfer_objects::statistic_id::StatisticId;
use web_api::types::LogRequest;

#[derive(Debug, FromForm)]
pub struct LogForm {
    pub statistic_id: u32,
    pub value: String,
    pub update_policy: Option<String>,
}

#[derive(Serialize)]
struct StatisticSelection<'a> {
    id: StatisticId,
    name: &'a str,
}

#[get("/log")]
pub fn show_log_interface(context: Context, flash: Option<FlashMessage<'_>>) -> Template {
    let statistics = lifestats_core::statistics::get_all_statistics(&context.0).unwrap();

    let statistic_list: Vec<StatisticSelection> = statistics
        .iter()
        .map(|s| StatisticSelection {
            id: s.id(),
            name: s.get_name(),
        })
        .collect();

    Template::render(
        "log_interface",
        context! {
            active_page: "logger",
            statistics: statistic_list,
            update_policies: vec![
                "Replace".to_string(),
                "Add".to_string(),
                "Average".to_string(),
            ],
            flash: flash.map(|flash| (flash.kind().to_string(), flash.message().to_string()))
        },
    )
}

#[post("/log", data = "<form>")]
pub async fn submit_log(context: Context, form: Form<LogForm>) -> Flash<Redirect> {
    let request = LogRequest {
        statistic_id: StatisticId(form.statistic_id),
        value: form.value.clone(),
        update_policy: form.update_policy.clone(),
        date: time::OffsetDateTime::now_utc().date(),
    };

    match log_statistic(&context.0, request).await {
        Ok(result) => Flash::success(Redirect::to(uri!(show_log_interface)), result.message),
        Err(e) => Flash::error(
            Redirect::to(uri!(show_log_interface)),
            format!("Error: {}", e),
        ),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::rocket;
    use rocket::http::{ContentType, Status};
    use rocket::local::blocking::Client;

    fn create_test_client() -> Client {
        Client::tracked(rocket()).expect("valid rocket instance")
    }

    #[test]
    fn test_invalid_statistic_id_format() {
        let client = create_test_client();
        let response = client
            .post("/log")
            .header(ContentType::Form)
            .body("statistic=not_a_number&value=10&update_policy=Replace")
            .dispatch();

        assert_eq!(response.status(), Status::UnprocessableEntity);
    }

    #[test]
    fn test_missing_value() {
        let client = create_test_client();
        let response = client
            .post("/log")
            .header(ContentType::Form)
            .body("statistic=1&update_policy=Replace")
            .dispatch();

        assert_eq!(response.status(), Status::UnprocessableEntity);
    }
}
