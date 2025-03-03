pub mod midi_device;

use crate::config::MidiLoggerConfig;
use crate::midi::midi_device::MidiDevice;
use anyhow::{Context, Result};
use heart_beat_logger::logger::http_logger::HttpLogger;
use heart_beat_logger::logger::logger_trait::TimeFrameLogger;
use heart_beat_logger::{HeartbeatConfig, HeartbeatLogger};
use log::info;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;
use testing_utilities::time_provider::{RealTimeProvider, TimeProvider};

pub enum LoggerMessage {
    Keypress,
    Stop,
}

pub struct MidiLogger {
    heartbeat: HeartbeatLogger<HttpLogger, RealTimeProvider>,
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
            Arc::new(RealTimeProvider),
            HttpLogger::new(config.statistic_id, &config.server_url)
                .context("Failed to create HTTP logger")?,
            HeartbeatConfig {
                idle_timeout: chrono::Duration::seconds(60),
                flush_interval: chrono::Duration::seconds(60),
            },
        );

        midi_device
            .connect(note_sender)
            .context("Failed to connect to MIDI input")?;

        Ok((
            Self {
                heartbeat,
                midi_device,
                message_rx: rx,
            },
            stop_sender,
        ))
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
