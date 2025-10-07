use anyhow::Result;
use lifestats_application::StatisticMetadata;
use std::io::{self, Write};

/// Terminal-based user interface for the MIDI logger
pub struct TerminalUI;

impl TerminalUI {
    pub fn new() -> Self {
        Self
    }

    /// Display available MIDI ports and prompt user to select one.
    /// Returns None if user quits, or Some(index) if a port is selected.
    pub fn select_midi_port(&self, ports: &[String]) -> Result<Option<usize>> {
        match ports.len() {
            0 => Err(anyhow::anyhow!("No MIDI input ports found")),
            1 => {
                println!("Only one MIDI port available, using: {}", ports[0]);
                Ok(Some(0))
            }
            _ => {
                println!("\nAvailable MIDI input ports:");
                for (i, port_name) in ports.iter().enumerate() {
                    println!("  {}: {}", i, port_name);
                }

                loop {
                    print!("\nSelect port number (or 'q' to quit): ");
                    io::stdout().flush()?;

                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    let input = input.trim();

                    if input.eq_ignore_ascii_case("q") {
                        return Ok(None);
                    }

                    match input.parse::<usize>() {
                        Ok(index) if index < ports.len() => {
                            return Ok(Some(index));
                        }
                        _ => {
                            println!(
                                "Invalid port number. Please enter a number between 0 and {}",
                                ports.len() - 1
                            );
                        }
                    }
                }
            }
        }
    }

    /// Display available statistics and prompt user to select one.
    /// Returns None if user quits, or Some(StatisticMetadata) if a statistic is selected.
    pub fn select_statistic(
        &self,
        statistics: &[StatisticMetadata],
    ) -> Result<Option<StatisticMetadata>> {
        if statistics.is_empty() {
            return Err(anyhow::anyhow!("No TimeSpans statistics available"));
        }

        println!("\nAvailable statistics:");
        for stat in statistics {
            println!("  ID {:3}: {}", stat.id().0, stat.name());
        }

        loop {
            print!("\nEnter statistic ID (or 'q' to quit): ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.eq_ignore_ascii_case("q") {
                return Ok(None);
            }

            match input.parse::<u32>() {
                Ok(id) => {
                    if let Some(stat) = statistics.iter().find(|s| s.id().0 == id) {
                        return Ok(Some(stat.clone()));
                    } else {
                        println!("Invalid ID. Please select from the list above.");
                    }
                }
                Err(_) => {
                    println!("Invalid input. Please enter a numeric ID.");
                }
            }
        }
    }

    /// Display a header message when the session starts
    pub fn show_session_header(&self, statistic_name: &str, port_name: &str) {
        println!("\n====================================");
        println!("MIDI Logger Session Started");
        println!("====================================");
        println!("Statistic: {}", statistic_name);
        println!("MIDI Port: {}", port_name);
        println!("\nPlaying notes will log time to the statistic.");
        println!("Press Ctrl+C to stop and save.\n");
    }

    /// Display a message when a MIDI beat is detected
    pub fn show_beat_message(&self) {
        println!("Beat detected!");
    }

    /// Display shutdown message
    pub fn show_shutdown_message(&self) {
        println!("\nShutting down and saving session...");
    }
}

impl Default for TerminalUI {
    fn default() -> Self {
        Self::new()
    }
}
