use midir::{MidiInput, MidiInputConnection, MidiInputPort};
use anyhow::{anyhow, Context, Result};
use crate::midi::LoggerMessage;
use std::sync::mpsc::{Sender};
use log::{debug};

pub struct MidiDevice {
    midi_input: Option<MidiInput>,
    selected_port: Option<MidiInputPort>,
    midi_connection: Option<MidiInputConnection<()>>,
}

impl MidiDevice {
    pub fn new() -> Result<MidiDevice> {
        Ok(Self {
            midi_input: Some(MidiInput::new("midir reading input")
                .context("Failed to create MIDI input")?),
            selected_port: None,
            midi_connection: None
        })
    }

    pub fn get_available_ports(&self) -> Vec<String> {
        self.midi_input.as_ref()
            .map(|input| input.ports().iter()
                .filter_map(|port| input.port_name(&port).ok())
                .collect())
            .unwrap_or_default()
    }

    pub fn select_port(&mut self, target_port_name: &str) -> Result<String> {
        let input = self.midi_input.as_ref()
            .ok_or(anyhow!("MIDI input not available"))?;

        let ports = input.ports();

        for port in ports {
            if let Ok(name) = input.port_name(&port) {
                if name == target_port_name {
                    self.selected_port = Some(port);
                    return Ok(name);
                }
            }
        }

        Err(anyhow!("MIDI port '{}' not found", target_port_name))
    }

    pub fn connect(
        &mut self,
        message_tx: Sender<LoggerMessage>
    ) -> Result<()> {
        let port = self.selected_port.as_ref().ok_or(anyhow!("No port selected."))?;

        let input = self.midi_input.take()
            .ok_or(anyhow!("MIDI input already connected"))?;

        let connection = input.connect(
            port,
            "midir-read-input",
            move |_stamp, message, _| {
                if Self::is_note(message) {
                    if let Err(e) = message_tx.send(LoggerMessage::Keypress) {
                        debug!("Failed to send keypress message: {}", e);
                    }
                }
            },
            (),
        ).map_err(|e| anyhow!("Failed to connect to MIDI input: {}", e))?;

        self.midi_connection = Some(connection);
        Ok(())
    }

    fn is_note(message: &[u8]) -> bool {
        message[0] & 0xF0 == 0x90
    }
}