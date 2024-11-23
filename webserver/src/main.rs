// webserver/src/main.rs
// Web server for managing statistics, providing endpoints for viewing plots,
// logging data, and managing goals through a web interface.

#[macro_use]
extern crate rocket;

pub mod logger;
mod heartbeat;
mod plots;
mod goal_edit;
mod logging;

use crate::logger::log_form::{show_log_interface, submit_log};
use rocket_dyn_templates::Template;
use crate::logger::log_api::log_statistic_api;
use crate::plots::plots_page::{serve_plot, show_plots_page};
use crate::goal_edit::goal_edit_page::{show_goals, update_goal};

#[launch]
fn rocket() -> _ {
    // logging::setup_logging().unwrap();

    rocket::build()
        .mount("/", routes![
            show_plots_page,
            serve_plot,
            show_log_interface,
            submit_log,
            log_statistic_api,
            show_goals,
            update_goal,
        ])
        // .manage(HeartbeatManager::new())
        .attach(Template::fairing())
}