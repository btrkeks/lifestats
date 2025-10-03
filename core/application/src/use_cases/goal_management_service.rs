use std::sync::Arc;

use anyhow::{bail, Context, Result};
use chrono::{Local, NaiveDate};
use events::event_port::{Event, EventPublisher};
use lifestats_domain::shared::ids::StatisticId;
use lifestats_domain::shared::percentage::Percentage;
use lifestats_domain::shared::time_frame_list::TimeFrameList;
use lifestats_domain::statistic::any_statistic::AnyStatistic;
use lifestats_domain::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_domain::statistic::traits::base_statistic::Statistic;
use lifestats_domain::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;
use lifestats_domain::statistic::value_objects::goals::time_spans_goal::TimeSpansGoal;
use serde_json;

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
        let SetGoalForDateCommand {
            statistic_id,
            date,
            target,
            extra_data_1,
        } = command;

        let statistic = self
            .statistic_repository
            .find_metadata_by_id(statistic_id)?;
        ensure_time_spans_statistic(&statistic, statistic_id)?;

        let goal = parse_time_spans_goal(&target, extra_data_1.as_deref())?;

        self.statistic_repository
            .save_goal::<TimeSpansGoal>(statistic_id, date, Some(goal))?;

        self.event_publisher
            .publish_event_async(Event::GoalUpdated(statistic_id));

        Ok(())
    }

    pub fn set_default_goal(&self, command: SetDefaultGoalCommand) -> Result<()> {
        let SetDefaultGoalCommand {
            statistic_id,
            target,
            extra_data_1,
        } = command;

        let mut statistic = self
            .statistic_repository
            .find_metadata_by_id(statistic_id)?;

        let goal = parse_time_spans_goal(&target, extra_data_1.as_deref())?;
        let start_date = Local::now().date_naive();

        let time_spans = as_time_spans_mut(&mut statistic, statistic_id)?;
        time_spans.set_new_default_goal(goal.clone(), start_date)?;

        self.statistic_repository.save_metadata(&statistic)?;
        self.statistic_repository
            .save_default_goal::<TimeSpansGoal>(statistic_id, goal)?;

        self.event_publisher
            .publish_event_async(Event::GoalUpdated(statistic_id));

        Ok(())
    }

    pub fn delete_goal_for_date(&self, command: DeleteGoalForDateCommand) -> Result<()> {
        let DeleteGoalForDateCommand { statistic_id, date } = command;

        let statistic = self
            .statistic_repository
            .find_metadata_by_id(statistic_id)?;
        ensure_time_spans_statistic(&statistic, statistic_id)?;

        self.statistic_repository
            .save_goal::<TimeSpansGoal>(statistic_id, date, None)?;

        self.event_publisher
            .publish_event_async(Event::GoalUpdated(statistic_id));

        Ok(())
    }

    pub fn delete_default_goal(&self, statistic_id: StatisticId) -> Result<()> {
        let statistic = self
            .statistic_repository
            .find_metadata_by_id(statistic_id)?;
        ensure_time_spans_statistic(&statistic, statistic_id)?;

        self.statistic_repository
            .delete_default_goal(statistic_id)?;

        self.event_publisher
            .publish_event_async(Event::GoalUpdated(statistic_id));

        Ok(())
    }

    pub fn get_goal_for_date(&self, query: GoalForDateQuery) -> Result<Option<GoalSnapshot>> {
        let statistic = self
            .statistic_repository
            .find_metadata_by_id(query.statistic_id)?;
        ensure_time_spans_statistic(&statistic, query.statistic_id)?;

        let goal = self
            .statistic_repository
            .find_goal::<TimeSpansGoal>(query.statistic_id, query.date)?;

        goal.map(|goal| snapshot_from_time_spans_goal(&goal))
            .transpose()
    }

    pub fn get_default_goal(&self, statistic_id: StatisticId) -> Result<Option<GoalSnapshot>> {
        let statistic = self
            .statistic_repository
            .find_metadata_by_id(statistic_id)?;
        ensure_time_spans_statistic(&statistic, statistic_id)?;

        let goal = self
            .statistic_repository
            .find_default_goal::<TimeSpansGoal>(statistic_id)?;

        goal.map(|goal| snapshot_from_time_spans_goal(&goal))
            .transpose()
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

fn ensure_time_spans_statistic(statistic: &AnyStatistic, id: StatisticId) -> Result<()> {
    if statistic.as_time_spans().is_some() {
        Ok(())
    } else {
        bail!("Statistic {} is not a time spans statistic", id)
    }
}

fn as_time_spans_mut(
    statistic: &mut AnyStatistic,
    id: StatisticId,
) -> Result<&mut TimeSpansStatistic> {
    match statistic {
        AnyStatistic::TimeSpans(inner) => Ok(inner),
        _ => bail!("Statistic {} is not a time spans statistic", id),
    }
}

fn parse_time_spans_goal(target: &str, extra_data_1: Option<&str>) -> Result<TimeSpansGoal> {
    let time_frames: TimeFrameList =
        serde_json::from_str(target).context("Failed to parse time frame list from goal target")?;

    let efficiency = extra_data_1
        .map(parse_percentage)
        .transpose()? // Result<Option<u8>>
        .unwrap_or(100);

    Ok(TimeSpansGoal::new(time_frames, Percentage::new(efficiency)))
}

fn parse_percentage(raw: &str) -> Result<u8> {
    let percentage: u8 = raw
        .trim()
        .parse()
        .with_context(|| format!("Failed to parse efficiency percentage from '{}'", raw))?;

    if percentage > 100 {
        bail!(
            "Efficiency percentage must be between 0 and 100, got {}",
            percentage
        );
    }

    Ok(percentage)
}

fn snapshot_from_time_spans_goal(goal: &TimeSpansGoal) -> Result<GoalSnapshot> {
    let target = serde_json::to_string(goal.target()).context("Failed to serialize goal target")?;
    let extra_data_1 = Some(goal.efficiency().as_num().to_string());

    Ok(GoalSnapshot {
        target,
        extra_data_1,
    })
}
