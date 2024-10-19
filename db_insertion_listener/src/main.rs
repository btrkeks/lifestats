use anyhow::Result;
use backend::db_events::{listen_for_db_event, Event};
use plotter_backend::regenerate_all_plots;
use std::process::Command;

fn update_wallpaper() -> bool {
    Command::new("sh")
        .arg("-c")
        .arg("~/.local/bin/update_wallpaper")
        .status()
        .expect("failed to update wallpaper")
        .success()
}

fn main() -> Result<()> {
    Ok(listen_for_db_event(|event| {
        match event {
            Event::RowUpdated(id) => {
                // TODO: Update only plots corresponding to id
                println!("Received RowUpdated event for id {}", id);

                regenerate_all_plots().expect("Could not regenerate plots");
                if !update_wallpaper() {
                    eprintln!("Failed to update wallpaper");
                }
            }
        }
    }))
}
