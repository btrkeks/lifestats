use std::sync::Arc;

use anyhow::{bail, Result};
use chrono::NaiveDate;
use events::event_port::EventPublisher;
use lifestats_domain::shared::ids::StatisticId;
use lifestats_domain::statistic::ports::statistic_repository::StatisticRepository;

#[derive(Clone)]
pub struct GoalManagementService<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    statistic_repository: Arc<R>,
    event_publisher: Arc<E>,
}

impl<R, E> GoalManagementService<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: EventPublisher + 'static,
{
    pub fn new(statistic_repository: Arc<R>, event_publisher: Arc<E>) -> Self {
        Self {
            statistic_repository,
            event_publisher,
        }
    }

    pub fn set_goal_for_date(&self, command: SetGoalForDateCommand) -> Result<()> {
        let _ = (&self.statistic_repository, &self.event_publisher, command);

        bail!("TODO 2.2: implement goal-for-date orchestration")
    }

    pub fn set_default_goal(&self, command: SetDefaultGoalCommand) -> Result<()> {
        let _ = (&self.statistic_repository, &self.event_publisher, command);

        bail!("TODO 2.2: implement default-goal orchestration")
    }

    pub fn delete_goal_for_date(&self, command: DeleteGoalForDateCommand) -> Result<()> {
        let _ = (&self.statistic_repository, &self.event_publisher, command);

        bail!("TODO 2.2: implement goal deletion orchestration")
    }

    pub fn delete_default_goal(&self, statistic_id: StatisticId) -> Result<()> {
        let _ = (
            &self.statistic_repository,
            &self.event_publisher,
            statistic_id,
        );

        bail!("TODO 2.2: implement default goal deletion orchestration")
    }

    pub fn get_goal_for_date(&self, query: GoalForDateQuery) -> Result<Option<GoalSnapshot>> {
        let _ = (&self.statistic_repository, query);

        bail!("TODO 2.2: implement goal query orchestration")
    }

    pub fn get_default_goal(&self, statistic_id: StatisticId) -> Result<Option<GoalSnapshot>> {
        let _ = (&self.statistic_repository, statistic_id);

        bail!("TODO 2.2: implement default goal query orchestration")
    }
}

#[derive(Debug, Clone)]
pub struct SetGoalForDateCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub target: String,
    pub extra_data_1: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetDefaultGoalCommand {
    pub statistic_id: StatisticId,
    pub target: String,
    pub extra_data_1: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeleteGoalForDateCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct GoalForDateQuery {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct GoalSnapshot {
    pub target: String,
    pub extra_data_1: Option<String>,
}
