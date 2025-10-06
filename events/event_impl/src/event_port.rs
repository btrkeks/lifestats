use anyhow::Result;
use lifestats_core::shared::ids::StatisticId;
use mockall::automock;
use serde::{Deserialize, Serialize};
use std::thread::JoinHandle;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Event {
    /// Signalizes that the logged value of StatisticId has changed
    ValueUpdated(StatisticId),
    /// Signalizes that the goal for StatisticId has been updated
    GoalUpdated(StatisticId),
    /// Signalizes that a new day has started, so all plots need to be updated
    NewDay,
}

#[automock]
pub trait EventPublisher: Send + Sync {
    fn publish_event_async(&self, event: Event);
    fn publish_event_sync(&self, event: Event) -> Result<()>;
}

pub trait EventSubscriber {
    fn listen_for_events(&self, event_handler: impl EventHandler) -> Result<JoinHandle<()>>;
}

// pub trait EventPubSub: EventPublisher + EventSubscriber {}

#[cfg_attr(test, automock)]
pub trait EventHandler: Send + 'static {
    fn handle_event(&mut self, event: Event);
}
