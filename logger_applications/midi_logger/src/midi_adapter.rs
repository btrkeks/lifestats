use anyhow::{anyhow, Context, Result};
use log::debug;
use midir::{MidiInput, MidiInputConnection};
use tokio::sync::mpsc;

/// High-level MIDI events that the application cares about.
/// This decouples the application from raw MIDI message formats.
#[derive(Debug, Clone)]
pub enum MidiEvent {
    /// A note-on event was received
    NoteOn,
}

/// Adapter for MIDI hardware interaction.
/// Translates low-level midir callbacks into async-friendly tokio channels.
pub struct MidiAdapter {
    midi_input: Option<MidiInput>,
    _connection: Option<MidiInputConnection<()>>,
}

impl MidiAdapter {
    /// Create a new MIDI adapter
    pub fn new() -> Result<Self> {
        let midi_input = MidiInput::new("midi_logger").context("Failed to create MIDI input")?;

        Ok(Self {
            midi_input: Some(midi_input),
            _connection: None,
        })
    }

    /// List all available MIDI input port names
    pub fn list_ports(&self) -> Result<Vec<String>> {
        let input = self
            .midi_input
            .as_ref()
            .ok_or_else(|| anyhow!("MIDI input not available"))?;

        let port_names: Vec<String> = input
            .ports()
            .iter()
            .filter_map(|port| input.port_name(port).ok())
            .collect();

        Ok(port_names)
    }

    /// Connect to a MIDI port by index and return a receiver for MIDI events.
    ///
    /// The midir callback runs on a separate thread, but tokio::sync::mpsc::Sender
    /// is Send + Sync, so it can be safely used from the callback thread.
    pub fn connect(mut self, port_index: usize) -> Result<mpsc::Receiver<MidiEvent>> {
        let input = self
            .midi_input
            .take()
            .ok_or_else(|| anyhow!("MIDI input already connected or not available"))?;

        let ports = input.ports();
        let port = ports
            .get(port_index)
            .ok_or_else(|| anyhow!("Invalid port index: {}", port_index))?;

        // Create an async channel for MIDI events
        let (tx, rx) = mpsc::channel(32);

        // Connect to the MIDI port with a callback
        // The callback runs on midir's internal thread, but tokio::sync::mpsc::Sender
        // is thread-safe and can be called from any thread
        let connection = input
            .connect(
                port,
                "midi_logger_connection",
                move |_timestamp, message, _| {
                    if Self::is_note_on(message) {
                        // Try to send the event; if the receiver is dropped, just log and continue
                        if let Err(e) = tx.try_send(MidiEvent::NoteOn) {
                            debug!("Failed to send MIDI event: {}", e);
                        }
                    }
                },
                (),
            )
            .map_err(|e| anyhow!("Failed to connect to MIDI port: {}", e))?;

        self._connection = Some(connection);

        Ok(rx)
    }

    /// Check if a MIDI message is a note-on event (status byte 0x9n)
    fn is_note_on(message: &[u8]) -> bool {
        !message.is_empty() && (message[0] & 0xF0 == 0x90)
    }
}
