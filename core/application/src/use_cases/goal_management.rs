use crate::app_context::AppContext;
use anyhow::Result;
use chrono::NaiveDate;
use events::event_port::Event;
use transfer_objects::raw_goal::RawGoal;
use lifestats_domain::shared::ids::StatisticId;

pub fn set_goal_for_date(
    app_context: &AppContext,
    statistic_id: StatisticId,
    date: NaiveDate,
    target: &str,
    extra_data_1: Option<&str>,
) -> Result<()> {
    let mut statistic = app_context.statistic_repository.get_by_id(statistic_id)?;
    statistic.set_goal_for_date_from_string(date, Some(target), extra_data_1)?;

    app_context
        .event_publisher
        .publish_event_async(Event::GoalUpdated(statistic_id));
    Ok(())
}

pub fn set_default_goal(
    app_context: &AppContext,
    statistic_id: StatisticId,
    target: &str,
    extra_data_1: Option<&str>,
) -> Result<()> {
    let mut statistic = app_context.statistic_repository.get_by_id(statistic_id)?;
    statistic.set_default_goal_from_string(Some(target), extra_data_1)?;

    app_context
        .event_publisher
        .publish_event_async(Event::GoalUpdated(statistic_id));
    Ok(())
}

pub fn delete_goal_for_date(
    app_context: &AppContext,
    statistic_id: StatisticId,
    date: NaiveDate,
) -> Result<()> {
    let mut statistic = app_context.statistic_repository.get_by_id(statistic_id)?;
    statistic.set_goal_for_date_from_string(date, None, None)?;

    app_context
        .event_publisher
        .publish_event_async(Event::GoalUpdated(statistic_id));
    Ok(())
}

pub fn delete_default_goal(app_context: &AppContext, statistic_id: StatisticId) -> Result<()> {
    let mut statistic = app_context.statistic_repository.get_by_id(statistic_id)?;
    statistic.set_default_goal_from_string(None, None)?;

    app_context
        .event_publisher
        .publish_event_async(Event::GoalUpdated(statistic_id));
    Ok(())
}

pub fn get_goal_for_date(
    app_context: &AppContext,
    statistic_id: StatisticId,
    date: NaiveDate,
) -> Result<Option<RawGoal>> {
    let mut statistic = app_context.statistic_repository.get_by_id(statistic_id)?;
    statistic.get_raw_goal_for_date(date)
}

pub fn get_default_goal(
    app_context: &AppContext,
    statistic_id: StatisticId,
) -> Result<Option<RawGoal>> {
    let mut statistic = app_context.statistic_repository.get_by_id(statistic_id)?;
    statistic.get_raw_default_goal()
}
