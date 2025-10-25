mod midi_adapter;
mod user_interface;

use anyhow::Result;
use lifestats_application::cqrs::QueryHandler;
use lifestats_application::use_cases::{
    queries::get_time_spans_metadata::{
        GetTimeSpansMetadataQuery, GetTimeSpansMetadataQueryHandler,
    },
    HeartbeatConfig, HeartbeatSession, RealTimeProvider,
};
use log::info;
use midi_adapter::MidiAdapter;
use std::sync::Arc;
use tokio::select;
use user_interface::TerminalUI;

/// Main MIDI logger application entry point.
/// This is a Driving Adapter that translates MIDI hardware events into application use case calls.
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    info!("Starting MIDI logger application");

    // Build all application services from the composition root
    let app = lifestats_bootstrap::build_services()?;

    // Create UI and hardware adapters
    let ui = TerminalUI::new();

    // Step 1: Select MIDI port
    let midi_adapter = MidiAdapter::new()?;
    let ports = midi_adapter.list_ports()?;
    let port_index = match ui.select_midi_port(&ports)? {
        Some(index) => index,
        None => {
            info!("User cancelled port selection");
            return Ok(());
        }
    };
    let selected_port_name = ports
        .get(port_index)
        .expect("Selected port index should be valid");

    // Step 2: Select statistic to log to
    let time_spans_statistics =
        GetTimeSpansMetadataQueryHandler::new(app.statistic_repository.clone())
            .handle(GetTimeSpansMetadataQuery)?;
    let statistic = match ui.select_statistic(&time_spans_statistics)? {
        Some(stat) => stat,
        None => {
            info!("User cancelled statistic selection");
            return Ok(());
        }
    };

    // Step 3: Create heartbeat session
    let mut session = HeartbeatSession::new(
        statistic.id(),
        app.statistic_repository.clone(),
        Arc::new(RealTimeProvider),
        HeartbeatConfig {
            idle_timeout: chrono::Duration::minutes(1),
            flush_interval: chrono::Duration::seconds(10),
        },
    );

    // Subscribe to session updates (start/stop events)
    let mut update_rx = session.subscribe_to_updates();

    // Step 4: Connect to MIDI port and create session
    let mut midi_session = midi_adapter.connect(port_index)?;

    // Step 5: Setup Ctrl+C handler
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);
    ctrlc::set_handler(move || {
        info!("Received Ctrl+C signal");
        let _ = shutdown_tx.try_send(());
    })?;

    // Display session header
    ui.show_session_header(statistic.name(), selected_port_name);

    // Main event loop: listen for MIDI events, session updates, and Ctrl+C
    loop {
        select! {
            Some(midi_event) = midi_session.recv() => {
                info!("Received MIDI event: {:?}", midi_event);
                session.beat();
                ui.show_beat_message();
            }
            Ok(update) = update_rx.recv() => {
                ui.show_session_update(&update);
            }
            Some(_) = shutdown_rx.recv() => {
                info!("Shutdown signal received");
                break;
            }
        }
    }

    // Graceful shutdown: stop the session to ensure final time frame is persisted
    ui.show_shutdown_message();
    session.stop();
    info!("MIDI logger stopped successfully");

    Ok(())
}
