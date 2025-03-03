mod config;
/// This application connects a midi device to the hearbeat logger.
/// It is used to track the time where the user played an instrument (e.g. a piano).
mod midi;
mod port_selection;

use crate::config::MidiLoggerConfig;
use crate::midi::midi_device::MidiDevice;
use crate::midi::{LoggerMessage, MidiLogger};
use crate::port_selection::PortSelector;
use anyhow::{Context, Result};
use heart_beat_logger::logger::http_logger::HttpLogger;
use log::{error, info, warn};
use std::io::{stdin, stdout, Write};
use std::sync::mpsc::Sender;
use std::time::Duration;
use transfer_objects::statistic_id::StatisticId;
use util::stdout_logging::init_logging;

const SERVER_URL: &str = "http://192.168.0.103:8000";
const HEARTBEAT_TIMEOUT_SECS: u64 = 60;

fn main() {
    init_logging();
    info!("Starting MIDI logger application");
    match run() {
        Ok(_) => info!("Application terminated successfully"),
        Err(err) => error!("Application error: {}", err),
    }
}

fn run() -> Result<()> {
    let config = MidiLoggerConfig::load().unwrap_or_else(|_| create_new_config());

    let mut midi_device = MidiDevice::new()?;
    select_midi_port(&mut midi_device)?;

    let (mut logger, stop_sender) = MidiLogger::new(midi_device, config)?;

    setup_ctrl_c(stop_sender)?;

    logger.run()
}

fn setup_ctrl_c(message_tx: Sender<LoggerMessage>) -> Result<()> {
    let tx = message_tx.clone();
    ctrlc::set_handler(move || {
        info!("Received Ctrl-C signal");
        tx.send(LoggerMessage::Stop).unwrap_or_else(|e| {
            error!("Failed to send stop message: {}", e);
        });
    })?;
    Ok(())
}

fn create_new_config() -> MidiLoggerConfig {
    // TODO: Ask the user to enter the config instead

    let config = MidiLoggerConfig::new(
        SERVER_URL,
        Duration::from_secs(HEARTBEAT_TIMEOUT_SECS),
        StatisticId(5),
    );

    if let Err(e) = config.save() {
        warn!("Failed to save configuration: {}", e);
    } else {
        info!("Created and saved new configuration");
    }

    config
}

fn select_midi_port(midi_device: &mut MidiDevice) -> Result<()> {
    let port_selector = PortSelector::default();
    let available_ports = midi_device.get_available_ports();

    let chosen_port_index = port_selector.select_port(&available_ports, select_port_interactive)?;
    let chosen_port = available_ports
        .get(chosen_port_index)
        .context("Invalid port index")?;

    midi_device.select_port(chosen_port)?;
    port_selector.save_port_selection(chosen_port)?;
    Ok(())
}

fn select_port_interactive(port_names: &[String]) -> Result<usize> {
    println!("\nAvailable input ports:");
    for (i, p) in port_names.iter().enumerate() {
        println!("{}: {}", i, p);
    }

    print!("Please select input port: ");
    stdout().flush()?;
    let mut input = String::new();
    stdin().read_line(&mut input)?;

    input.trim().parse().context("Invalid port number")
}
