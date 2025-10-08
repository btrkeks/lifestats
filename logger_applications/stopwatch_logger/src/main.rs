mod ui;

use crate::ui::terminal_interface::TerminalInterface;
use crate::ui::user_interface_trait::{UserEvent, UserInput, UserInterface};
use anyhow::Result;
use lifestats_application::cqrs::QueryHandler;
use lifestats_application::use_cases::{
    queries::get_time_spans_metadata::{
        GetTimeSpansMetadataQuery, GetTimeSpansMetadataQueryHandler,
    },
    HeartbeatConfig, HeartbeatSession, HeartbeatUpdate, RealTimeProvider,
};
use lifestats_application::{StatisticMetadata, StatisticRepository};
use std::sync::Arc;
use tokio::select;

async fn run_stopwatch<I, R>(
    statistic_repository: Arc<R>,
    ui: &mut I,
    metadata: StatisticMetadata,
) -> Result<()>
where
    I: UserInterface,
    R: StatisticRepository + Send + Sync + 'static,
{
    let mut session = HeartbeatSession::new(
        metadata.id(),
        statistic_repository,
        Arc::new(RealTimeProvider),
        HeartbeatConfig {
            idle_timeout: chrono::Duration::days(1),
            flush_interval: chrono::Duration::minutes(1),
        },
    );

    let mut session_rx = session.subscribe_to_updates();
    let mut ui_rx = ui.run_event_loop()?;

    ui.show_stopwatch_header(&metadata)?;
    session.beat();

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

fn ask_for_valid_statistic<I, R>(
    statistic_repository: &Arc<R>,
    ui: &mut I,
) -> Result<Option<StatisticMetadata>>
where
    I: UserInterface,
    R: StatisticRepository + Send + Sync + 'static,
{
    let handler = GetTimeSpansMetadataQueryHandler::new(Arc::clone(statistic_repository));
    let time_spans_statistics = handler.handle(GetTimeSpansMetadataQuery)?;

    loop {
        let user_input = ui.show_choose_statistic_dialog(&time_spans_statistics)?;

        match user_input {
            UserInput::Number(id) => {
                if let Some(stat) = time_spans_statistics.iter().find(|s| s.id().0 == id) {
                    return Ok(Some(stat.clone()));
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

async fn run<I, R>(statistic_repository: Arc<R>, mut ui: I) -> Result<()>
where
    I: UserInterface,
    R: StatisticRepository + Send + Sync + 'static,
{
    ui.init()?;

    let result = async {
        let stat_metadata = ask_for_valid_statistic(&statistic_repository, &mut ui)?;

        if let Some(metadata) = stat_metadata {
            run_stopwatch(Arc::clone(&statistic_repository), &mut ui, metadata).await?;
        }

        Ok(())
    }
    .await;

    ui.close()?;
    result
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let app = lifestats_bootstrap::build_services()?;
    let user_interface = TerminalInterface::new();
    run(app.statistic_repository, user_interface).await
}
