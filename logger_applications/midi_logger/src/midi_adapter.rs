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
}

/// Active MIDI session that keeps the connection alive and provides access to events.
/// Dropping this struct will close the MIDI connection.
pub struct MidiSession {
    /// Connection must be kept alive for callbacks to continue working
    _connection: MidiInputConnection<()>,
    /// Channel receiver for MIDI events
    rx: mpsc::Receiver<MidiEvent>,
}

impl MidiSession {
    /// Receive the next MIDI event (async)
    pub async fn recv(&mut self) -> Option<MidiEvent> {
        self.rx.recv().await
    }
}

impl MidiAdapter {
    /// Create a new MIDI adapter
    pub fn new() -> Result<Self> {
        let midi_input = MidiInput::new("midi_logger").context("Failed to create MIDI input")?;

        Ok(Self {
            midi_input: Some(midi_input),
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

    /// Connect to a MIDI port by index and return an active session.
    ///
    /// The returned MidiSession must be kept alive for the connection to remain active.
    /// The midir callback runs on a separate thread, but tokio::sync::mpsc::Sender
    /// is Send + Sync, so it can be safely used from the callback thread.
    pub fn connect(mut self, port_index: usize) -> Result<MidiSession> {
        use log::info;

        let input = self
            .midi_input
            .take()
            .ok_or_else(|| anyhow!("MIDI input already connected or not available"))?;

        let ports = input.ports();
        let port = ports
            .get(port_index)
            .ok_or_else(|| anyhow!("Invalid port index: {}", port_index))?;

        info!("Connecting to MIDI port index {}", port_index);

        // Create an async channel for MIDI events
        let (tx, rx) = mpsc::channel(32);

        // Connect to the MIDI port with a callback
        // The callback runs on midir's internal thread, but tokio::sync::mpsc::Sender
        // is thread-safe and can be called from any thread
        let connection = input
            .connect(
                port,
                "midi_logger_connection",
                move |timestamp, message, _| {
                    debug!(
                        "MIDI callback triggered - timestamp: {}, message: {:?}",
                        timestamp, message
                    );

                    if Self::is_note_on(message) {
                        debug!("Note-on detected, sending to channel");
                        // Try to send the event; if the receiver is dropped, just log and continue
                        if let Err(e) = tx.try_send(MidiEvent::NoteOn) {
                            debug!("Failed to send MIDI event: {}", e);
                        }
                    } else {
                        debug!("Message is not a note-on, ignoring");
                    }
                },
                (),
            )
            .map_err(|e| anyhow!("Failed to connect to MIDI port: {}", e))?;

        info!("MIDI connection established successfully");

        Ok(MidiSession {
            _connection: connection,
            rx,
        })
    }

    /// Check if a MIDI message is a note-on event (status byte 0x9n)
    fn is_note_on(message: &[u8]) -> bool {
        !message.is_empty() && (message[0] & 0xF0 == 0x90)
    }
}
