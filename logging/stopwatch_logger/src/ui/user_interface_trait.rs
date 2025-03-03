use anyhow::Result;
use tokio::sync::mpsc;
use transfer_objects::statistic_metadata::StatisticMetadata;

pub trait UserInterface: Send + Sync {
    fn init(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
    fn show_stopwatch_header(&mut self, metadata: &StatisticMetadata) -> Result<()>;
    fn update_stopwatch_state(
        &mut self,
        is_running: bool,
        elapsed_time: chrono::Duration,
    ) -> Result<()>;
    fn show_choose_statistic_dialog(
        &mut self,
        statistics: &[StatisticMetadata],
    ) -> Result<UserInput>;
    fn show_invalid_id_message(&mut self, input: u32) -> Result<()>;

    // Now returns a receiver for events instead of polling
    fn run_event_loop(&self) -> Result<mpsc::Receiver<UserEvent>>;
}

#[derive(Debug, Eq, PartialEq)]
pub enum UserInput {
    Number(u32),
    Quit,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum UserEvent {
    Toggle,
    Quit,
    // Other,
}
