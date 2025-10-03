use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use chrono::{Local, NaiveDate};
use events::event_port::{Event, EventPublisher};
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

    //===========================================================================
    // POLYMORPHIC METHODS (Generic for all statistic types)
    //===========================================================================

    /// Set a goal for a specific date using primitive data (generic for all types)
    pub fn set_goal(&self, command: SetGoalCommand) -> Result<()> {
        // 1. Fetch the statistic metadata
        let statistic = self
            .statistic_repository
            .find_metadata_by_id(command.statistic_id)?;

        // 2. Tell the domain object to create a goal from the raw data
        let goal_json = statistic.create_goal_from_primitives(&command.goal_data)?;

        // 3. Save the resulting JSON via the repository
        self.statistic_repository.save_goal_json(
            command.statistic_id,
            command.date,
            Some(goal_json),
        )?;

        self.event_publisher
            .publish_event_async(Event::GoalUpdated(command.statistic_id));

        Ok(())
    }

    /// Set a new default goal using primitive data (generic for all types)
    pub fn set_default_goal_generic(&self, command: SetDefaultGoalGenericCommand) -> Result<()> {
        // 1. Fetch the statistic metadata
        let mut statistic = self
            .statistic_repository
            .find_metadata_by_id(command.statistic_id)?;

        // 2. Tell the domain object to set its default goal from the raw data
        let start_date = Local::now().date_naive();
        let goal_json =
            statistic.set_default_goal_from_primitives(&command.goal_data, start_date)?;

        // 3. Save the updated metadata and default goal
        self.statistic_repository.save_metadata(&statistic)?;
        self.statistic_repository
            .save_default_goal_json(command.statistic_id, goal_json)?;

        self.event_publisher
            .publish_event_async(Event::GoalUpdated(command.statistic_id));

        Ok(())
    }

    /// Delete a goal for a specific date (generic for all types)
    pub fn delete_goal(&self, command: DeleteGoalCommand) -> Result<()> {
        self.statistic_repository
            .save_goal_json(command.statistic_id, command.date, None)?;

        self.event_publisher
            .publish_event_async(Event::GoalUpdated(command.statistic_id));

        Ok(())
    }
}

//===========================================================================
// POLYMORPHIC COMMAND STRUCTURES (Generic for all statistic types)
//===========================================================================

/// Command for setting a goal for a specific date (generic)
#[derive(Debug, Clone)]
pub struct SetGoalCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub goal_data: HashMap<String, String>,
}

/// Command for setting a default goal (generic)
#[derive(Debug, Clone)]
pub struct SetDefaultGoalGenericCommand {
    pub statistic_id: StatisticId,
    pub goal_data: HashMap<String, String>,
}

/// Command for deleting a goal for a specific date (generic)
#[derive(Debug, Clone)]
pub struct DeleteGoalCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
}

//===========================================================================
// LEGACY COMMAND STRUCTURES (TimeSpans-specific)
//===========================================================================

/// Legacy command for setting TimeSpans goals for a date
#[derive(Debug, Clone)]
pub struct SetGoalForDateCommand {
    pub statistic_id: StatisticId,
    pub date: NaiveDate,
    pub target: String,
    pub extra_data_1: Option<String>,
}

/// Legacy command for setting TimeSpans default goals
#[derive(Debug, Clone)]
pub struct SetDefaultGoalCommand {
    pub statistic_id: StatisticId,
    pub target: String,
    pub extra_data_1: Option<String>,
}

/// Legacy command for deleting goals for a date
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
