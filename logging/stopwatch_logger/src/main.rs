// stopwatch/src/main.rs
// Stopwatch application for tracking time spent on activities. Provides an interactive
// CLI interface with play/pause functionality and integration with the statistics system.

#![feature(duration_constructors)]
mod ui;

use crate::ui::{clear_screen, print_elapsed_time};
use anyhow::{Context, Result};
use chrono::Duration;
use chrono::{DateTime, Local};
use crossterm::cursor::{Hide, Show};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
    ExecutableCommand,
};
use heart_beat_logger::HeartbeatLogger;
use std::io::{self, Write};
use std::time::Duration as StdDuration;
use backend::statistics::objects::StatisticId;
use backend::statistics::statistic_repository::{get_all_time_amount_statistics, get_all_time_spans_statistics};
use backend::statistics::type_definitions::{Statistic};
use backend::statistics::type_definitions::time_amount::TimeAmountStatistic;

struct Stopwatch {
    heartbeat_logger: HeartbeatLogger,
    previously_elapsed: Duration, // TODO: Implement more cleanly
    running_since: Option<DateTime<Local>>, //       - " -
    is_running: bool
}

impl Stopwatch {
    pub fn new(stat_id: StatisticId) -> Self {
        Stopwatch {
            heartbeat_logger: HeartbeatLogger::new_with_local_logger(stat_id, std::time::Duration::from_days(1)),
            previously_elapsed: Duration::zero(),
            running_since: None,
            is_running: false
        }
    }

    fn is_running(&self) -> bool {
        self.is_running
    }

    pub fn toggle(&mut self) {
        if self.is_running() {
            self.stop()
        } else {
            self.start()
        }
    }

    pub fn start(&mut self) {
        self.is_running = true;
        self.heartbeat_logger.beat();
        self.running_since = Some(Local::now());
    }

    pub fn stop(&mut self) {
        self.is_running = false;
        self.heartbeat_logger.stop();

        if let Some(running_since) = self.running_since.take() {
            self.previously_elapsed += Local::now().signed_duration_since(running_since);
        }
    }

    pub fn get_elapsed_time(&self) -> Duration {
        let current_elapsed = if let Some(running_since) = self.running_since {
            Local::now().signed_duration_since(running_since)
        } else {
            Duration::zero()
        };

        current_elapsed + self.previously_elapsed
    }
}

fn run_stopwatch(stat_id: StatisticId, stat_name: &str) -> Result<()> {
    enable_raw_mode().context("Failed to enable raw mode")?;
    io::stdout()
        .execute(Hide)
        .context("Failed to hide cursor")?;

    clear_screen();
    println!(
        "Stopwatch for '{}'. Press SPACE to pause/resume, 'q' to quit.\n",
        stat_name
    );

    let mut stopwatch = Stopwatch::new(stat_id);
    stopwatch.start();
    loop {
        if event::poll(StdDuration::from_millis(400)).context("Failed to poll for events")? {
            if let Event::Key(key_event) = event::read().context("Failed to read event")? {
                match key_event.code {
                    KeyCode::Char(' ') => {
                        stopwatch.toggle();
                        ui::update_state(stopwatch.is_running());
                    }
                    KeyCode::Char('q') => {
                        stopwatch.stop();
                        break;
                    }
                    _ => {}
                }
            }
        }

        if stopwatch.is_running() {
            print_elapsed_time(stopwatch.get_elapsed_time());
        }
    }

    disable_raw_mode().context("Failed to disable raw mode")?;
    io::stdout()
        .execute(Show)
        .context("Failed to show cursor")?;
    println!("\nStopwatch ended");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let amount_statistics: Vec<TimeAmountStatistic> = get_all_time_amount_statistics()?;
    let time_spans_statistics = get_all_time_spans_statistics()?;

    println!("Available statistics:");
    amount_statistics
        .iter()
        .for_each(|stat| println!("id: {:3}, name: {}", stat.get_id(), stat.get_name()));
    time_spans_statistics
        .iter()
        .for_each(|stat| println!("id: {:3}, name: {}", stat.get_id(), stat.get_name()));

    loop {
        print!("\nEnter the ID of the statistic you want to log (or 'q' to quit): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let input = input.trim();

        if input.eq_ignore_ascii_case("q") {
            break;
        }

        match input.parse::<u32>() {
            Ok(id_num) => {
                let id = StatisticId(id_num);
                if let Some(stat) = amount_statistics.iter().find(|s| s.get_id() == id) {
                    run_stopwatch(stat.get_id(), stat.get_name())?;
                    break;
                } else if let Some(stat) = time_spans_statistics.iter().find(|s| s.get_id() == id) {
                    run_stopwatch(stat.get_id(), stat.get_name())?;
                    break;
                } else {
                    println!("No AmountStatistic found with ID: {}", id);
                }
            }
            Err(_) => println!("Invalid input. Please enter a valid ID number or 'q' to quit."),
        }
    }

    println!("Goodbye!");
    Ok(())
}
