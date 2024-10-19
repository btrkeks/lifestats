use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use anyhow::{anyhow, Result};
use backend::statistics::Statistic;
use backend::{db_ops, statistic_getter::get_statistic_by_id};
use chrono::{Local, NaiveDate};
use log::info;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct LogRequest {
    id: u32,
    date: Option<NaiveDate>,
    value: String,
    update_policy: Option<String>,
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
}

async fn log_statistic(data: web::Json<LogRequest>) -> impl Responder {
    match log_statistic_internal(data.into_inner()) {
        Ok(_) => HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: "Statistic logged successfully".to_string(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse {
            success: false,
            message: format!("Failed to log statistic: {}", e),
        }),
    }
}

fn log_statistic_internal(data: LogRequest) -> Result<()> {
    let statistic =
        get_statistic_by_id(data.id)?.ok_or_else(|| anyhow!("Id {} does not exist", data.id))?;
    let conn = db_ops::open_db()?;
    let date = data.date.unwrap_or_else(|| Local::now().date_naive());
    statistic.log_from_string(&conn, date, &data.value, data.update_policy.as_deref())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    info!("Starting web server");

    HttpServer::new(|| App::new().route("/log", web::post().to(log_statistic)))
        .bind(("127.0.0.1", 8080))?
        .bind(("192.168.0.103", 8080))?
        .run()
        .await
}
