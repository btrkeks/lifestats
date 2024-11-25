use rocket::post;
use rocket::get;
use rocket::form::Form;
use rocket::response::{Flash, Redirect};
use rocket::serde::Serialize;
use rocket_dyn_templates::{context, Template};
use rocket::request::FlashMessage;
use backend::statistics::objects::StatisticId;
use backend::statistics::statistic_repository::get_all_statistics;
use backend::statistics::type_definitions::Statistic;
use api_client::types::LogRequest;
use crate::logger::log_core::{log_statistic};

#[derive(Debug, FromForm)]
pub struct LogForm {
    pub statistic_id: StatisticId,
    pub value: String,
    pub update_policy: Option<String>,
}

#[derive(Serialize)]
struct StatisticSelection<'a> {
    id: StatisticId,
    name: &'a str,
}

#[get("/log")]
pub fn show_log_interface(flash: Option<FlashMessage<'_>>) -> Template {
    let statistics = get_all_statistics().unwrap_or_default();

    let statistic_list: Vec<StatisticSelection> = statistics
        .iter()
        .map(|s| StatisticSelection {
            id: s.get_id(),
            name: s.get_name(),
        })
        .collect();

    Template::render("log_interface", context! {
        active_page: "logger",
        statistics: statistic_list,
        update_policies: vec![
            "Replace".to_string(),
            "Add".to_string(),
            "Average".to_string(),
        ],
        flash: flash.map(|flash| (flash.kind().to_string(), flash.message().to_string()))
    })
}

#[post("/log", data = "<form>")]
pub async fn submit_log(form: Form<LogForm>) -> Flash<Redirect> {
    let request = LogRequest {
        statistic_id: form.statistic_id,
        value: form.value.clone(),
        update_policy: form.update_policy.clone(),
        date: time::OffsetDateTime::now_utc().date(),
    };

    match log_statistic(request).await {
        Ok(result) => Flash::success(
            Redirect::to(uri!(show_log_interface)),
            result.message,
        ),
        Err(e) => Flash::error(
            Redirect::to(uri!(show_log_interface)),
            format!("Error: {}", e),
        ),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rocket::local::blocking::Client;
    use rocket::http::{ContentType, Status};
    use crate::rocket;

    fn create_test_client() -> Client {
        Client::tracked(rocket()).expect("valid rocket instance")
    }

    #[test]
    fn test_log_interface_renders() {
        let client = create_test_client();
        let response = client.get("/log").dispatch();

        assert_eq!(response.status(), Status::Ok);
        assert!(response.into_string().unwrap().contains("Log Statistic"));
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