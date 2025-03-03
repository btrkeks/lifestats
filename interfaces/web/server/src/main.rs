/// Description:
/// This appliaction is a web interface to the core application.
/// It provides an api through the /api/ endpoints and a graphical user interface at /

#[macro_use]
extern crate rocket;

mod goal_edit;
mod guards;
pub mod logger;
mod heartbeat;
mod cors;

use crate::goal_edit::goal_edit_page::{
    delete_goal_api, get_current_goal, set_goal_api, show_goals,
};
use crate::guards::Context;
use crate::logger::log_api::log_statistic_api;
use crate::logger::log_form::{show_log_interface, submit_log};
use lifestats_core::app_context::{build_application, AppContext};
use rocket::request::FromRequest;
use rocket_dyn_templates::Template;
use std::sync::Arc;
use heartbeat::manager::HeartbeatManager;
use rocket_cors::{AllowedHeaders, AllowedOrigins, Cors};

#[launch]
fn rocket() -> _ {
    let app_context = build_application().expect("Failed to build application context");
    let app_context = Arc::new(app_context);
    let heartbeat_manager = HeartbeatManager::new(Arc::clone(&app_context));

    rocket::build()
        .manage(Context(app_context))
        .manage(heartbeat_manager)
        .mount(
            "/",
            routes![
                show_log_interface,
                submit_log,
                log_statistic_api,
                set_goal_api,
                delete_goal_api,
                show_goals,
                get_current_goal,
                heartbeat::api::ping_heartbeat,
            ],
        )
        .attach(Template::fairing())
        .attach(cors::create_cors().expect("CORS configuration failed"))
}
