pub mod object;
pub mod midi_device;

use anyhow::{Context, Result};
use heart_beat_logger::{HeartbeatLogger, http_logger::HttpLogger};
use log::{info};
use std::sync::mpsc::{channel, Receiver, Sender};
use crate::config::MidiLoggerConfig;
use crate::midi::midi_device::MidiDevice;

pub enum LoggerMessage {
    Keypress,
    Stop,
}

pub struct MidiLogger {
    heartbeat: HeartbeatLogger,
    midi_device: MidiDevice,
    message_rx: Receiver<LoggerMessage>,
}

impl MidiLogger {
    pub fn new(
        mut midi_device: MidiDevice,
        config: MidiLoggerConfig,
    ) -> Result<(Self, Sender<LoggerMessage>)> {
        let (note_sender, rx) = channel();
        let stop_sender = note_sender.clone();

        let heartbeat = HeartbeatLogger::new(
            config.statistic_id,
            config.heartbeat_timeout,
            HttpLogger::new(&config.server_url).context("Failed to create HTTP logger")?,
        );

        midi_device.connect(note_sender).context("Failed to connect to MIDI input")?;

        Ok((Self {
            heartbeat,
            midi_device,
            message_rx: rx,
        }, stop_sender))
    }

    pub fn run(&mut self) -> Result<()> {
        info!("Starting MIDI logger loop");

        while let Ok(message) = self.message_rx.recv() {
            match message {
                LoggerMessage::Keypress => {
                    self.heartbeat.beat();
                }
                LoggerMessage::Stop => {
                    info!("Received stop signal");
                    break;
                }
            }
        }

        info!("Stopping heartbeat logger");
        self.heartbeat.stop();
        Ok(())
    }
}