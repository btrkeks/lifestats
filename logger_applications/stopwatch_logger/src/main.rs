// stopwatch/src/main.rs
// Stopwatch application for tracking time spent on activities. Provides an interactive
// CLI interface with play/pause functionality and integration with the statistics system.
#![feature(duration_constructors)]

mod ui;

use crate::ui::terminal_interface::TerminalInterface;
use crate::ui::user_interface_trait::{UserEvent, UserInput, UserInterface};
use anyhow::Result;
use domain_models::statistic::statistic_metadata::StatisticMetadata;
use lifestats_application::use_cases::{
    HeartbeatConfig, HeartbeatSession, HeartbeatUpdate, RealTimeProvider,
};
use lifestats_core::app_context::{build_application, AppContext};
use lifestats_core::statistics::get_time_spans_metadata;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tokio::select;
use transfer_objects::statistic_id::StatisticId;

async fn run_stopwatch<I: UserInterface>(
    app_context: AppContext,
    ui: &mut I,
    metadata: StatisticMetadata,
) -> Result<()> {
    // Create HeartbeatSession directly using the new application service
    let mut session = HeartbeatSession::new(
        metadata.id(),
        app_context.statistic_repository.clone(),
        Arc::new(RealTimeProvider),
        HeartbeatConfig {
            idle_timeout: chrono::Duration::days(1), // Long timeout for manual stopwatch
            flush_interval: chrono::Duration::minutes(1),
        },
    );

    let mut session_rx = session.subscribe_to_updates();
    let mut ui_rx = ui.run_event_loop()?;

    ui.show_stopwatch_header(&metadata)?;
    session.beat(); // Start the session

    loop {
        select! {
            Some(event) = ui_rx.recv() => {
                match event {
                    UserEvent::Toggle => {
                        if session.is_running() {
                            session.stop();
                        } else {
                            session.beat();
                        }
                    }
                    UserEvent::Quit => {
                        session.stop();
                        break;
                    }
                }
            }
            Some(update) = session_rx.recv() => {
                match update {
                    HeartbeatUpdate::DurationChanged(elapsed) => {
                        ui.update_stopwatch_state(session.is_running(), elapsed)?;
                    }
                    HeartbeatUpdate::Stopped => {
                        ui.update_stopwatch_state(false, session.total_time_elapsed())?;
                    }
                    HeartbeatUpdate::Started => {
                        ui.update_stopwatch_state(true, session.total_time_elapsed())?;
                    }
                }
            }
        }
    }

    Ok(())
}

fn ask_for_valid_statistic<I: UserInterface>(
    app_context: &AppContext,
    ui: &mut I,
) -> Result<Option<StatisticMetadata>> {
    let time_spans_statistics = get_time_spans_metadata(app_context)?;

    loop {
        let user_input = ui.show_choose_statistic_dialog(&time_spans_statistics)?;

        match user_input {
            UserInput::Number(id) => {
                let stat_id = StatisticId(id);
                if let Some(index) = time_spans_statistics.iter().position(|s| s.id() == stat_id) {
                    return Ok(Some(Vec::from(time_spans_statistics).swap_remove(index)));
                } else {
                    ui.show_invalid_id_message(id)?;
                }
            }
            UserInput::Quit => {
                return Ok(None);
            }
        }
    }
}

async fn run<I: UserInterface>(app_context: AppContext, mut ui: I) -> Result<()> {
    ui.init()?;

    // Properly close the UI if an error occurs
    let result = async {
        let stat_metadata = ask_for_valid_statistic(&app_context, &mut ui)?;

        if let Some(metadata) = stat_metadata {
            run_stopwatch(app_context, &mut ui, metadata).await?;
        }

        Ok(())
    }
    .await;

    ui.close()?;
    result
}

#[tokio::main]
async fn main() -> Result<()> {
    let app_context = build_application()?;
    let user_interface = TerminalInterface::new();
    run(app_context, user_interface).await
}
