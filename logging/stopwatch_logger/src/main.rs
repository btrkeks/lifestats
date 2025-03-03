// stopwatch/src/main.rs
// Stopwatch application for tracking time spent on activities. Provides an interactive
// CLI interface with play/pause functionality and integration with the statistics system.
#![feature(duration_constructors)]

mod stopwatch;
mod ui;

use crate::stopwatch::{Stopwatch, StopwatchConfig};
use crate::ui::terminal_interface::TerminalInterface;
use crate::ui::user_interface_trait::{UserEvent, UserInput, UserInterface};
use anyhow::Result;
use heart_beat_logger::HeartbeatUpdate;
use lifestats_core::app_context::{build_application, AppContext};
use lifestats_core::statistics::get_time_spans_metadata;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use testing_utilities::time_provider::RealTimeProvider;
use tokio::select;
use transfer_objects::statistic_id::StatisticId;
use transfer_objects::statistic_metadata::StatisticMetadata;

async fn run_stopwatch<I: UserInterface>(
    app_context: AppContext,
    ui: &mut I,
    metadata: StatisticMetadata,
) -> Result<()> {
    let mut stopwatch = Stopwatch::new(
        app_context,
        Arc::new(RealTimeProvider),
        StopwatchConfig {
            statistic_id: metadata.id(),
            log_flush_interval: chrono::Duration::minutes(1),
        },
    );

    let mut stopwatch_rx = stopwatch.subscribe_to_updates();
    let mut ui_rx = ui.run_event_loop()?;

    ui.show_stopwatch_header(&metadata)?;
    stopwatch.start();

    loop {
        select! {
            Some(event) = ui_rx.recv() => {
                match event {
                    UserEvent::Toggle => {
                        stopwatch.toggle();
                    }
                    UserEvent::Quit => {
                        stopwatch.stop();
                        break;
                    }
                }
            }
            Some(update) = stopwatch_rx.recv() => {
                match update {
                    HeartbeatUpdate::DurationChanged(elapsed) => {
                        ui.update_stopwatch_state(stopwatch.is_running(), elapsed)?;
                    }
                    HeartbeatUpdate::Stopped => {
                        ui.update_stopwatch_state(false, stopwatch.get_elapsed_time())?;
                    }
                    HeartbeatUpdate::Started => {
                        ui.update_stopwatch_state(true, stopwatch.get_elapsed_time())?;
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
