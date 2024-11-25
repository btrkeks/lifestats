// db_insertion_listener/src/main.rs
// Background service that listens for database events and triggers plot regeneration
// and wallpaper updates when statistics are modified.

use anyhow::Result;
use backend::database::db_events::{listen_for_db_event, Event};
use backend::statistics::objects::StatisticId;
use plotting::regenerate_plots_for_id;
use std::process::Command;

fn update_wallpaper(_id: StatisticId) -> Result<(), ()> {
    let success = Command::new("sh")
        .arg("-c")
        .arg("~/.local/bin/update_wallpaper")
        .status()
        .expect("failed to update wallpaper")
        .success();

    if success { Ok(()) } else { Err(()) }
}

fn main() {
    listen_for_db_event(|event| {
        match event {
            Event::ValueUpdated(id) | Event::GoalUpdated(id) => {
                println!("Received event for id {}", id);

                if regenerate_plots_for_id(id).is_err() {
                    eprintln!("Could not regenerate plots for id {}", id);
                }

                if update_wallpaper(id).is_err() {
                    eprintln!("Failed to update wallpaper for id {}", id);
                }
            }
        }
    })
}
