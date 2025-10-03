use crate::app_context::AppContext;
use anyhow::{anyhow, Context, Result};
use chrono::{NaiveDate, NaiveTime};
use lifestats_domain::shared::time_frame_list::TimeFrame;
use lifestats_domain::statistic::statistic_types::statistic_types::logable_statistics::update_policy::time_frame_list::TimeFrameListUpdatePolicy;
use lifestats_domain::statistic::statistic::Statistic;
use lifestats_domain::statistic::statistic_types::any_statistic::AnyStatistic;
use events::event_port::Event;
use lifestats_domain::shared::ids::StatisticId;

/// Value Formatting guidelines:
/// - Times should be in the format "HH:MM:SS" or "HH:MM", e.g. "12:30:00", "12:30"
/// - Durations should end with a unit (s, m, or h) and be a number, e.g.: "5m", "2h", "30s"
///   uppercase is allowed
/// - Time frames should be in the format "HH:MM-HH:MM" or "HH:MM:SS-HH:MM:SS", e.g. "12:30-13:30", "12:30:00-13:30:00"
pub fn log_statistic(
    app_context: &AppContext,
    statistic_id: StatisticId,
    date: NaiveDate,
    value: &str,
    update_policy: Option<&str>,
) -> Result<()> {
    let statistic = app_context.statistic_repository.get_by_id(statistic_id)?;
    statistic.log_from_string(date, value, update_policy)?;

    app_context
        .event_publisher
        .publish_event_async(Event::ValueUpdated(statistic_id));
    Ok(())
}

pub fn log_time_frame(
    app_context: &AppContext,
    statistic_id: StatisticId,
    date: NaiveDate,
    start_time: NaiveTime,
    end_time: NaiveTime,
    update_policy: Option<TimeFrameListUpdatePolicy>,
) -> Result<()> {
    let statistic = app_context.statistic_repository.get_by_id(statistic_id)?;

    if let AnyStatistic::TimeSpans(s) = statistic {
        let new_value = TimeFrame::new(start_time, end_time).with_context(|| {
            format!(
                "Failed to create TimeFrame from start time: {} and end time: {}",
                start_time, end_time
            )
        })?;

        s.log_value(date, new_value, update_policy.unwrap_or_default())
            .with_context(|| {
                format!(
                    "Failed to log time frame for statistic with ID: {}",
                    statistic_id
                )
            })?;
    } else {
        return Err(anyhow!(
            "Statistic with id {} doesn't store time spans!",
            statistic_id
        ));
    }

    app_context
        .event_publisher
        .publish_event_async(Event::ValueUpdated(statistic_id));
    Ok(())
}

// TODO: Add a test that logging properly triggers events
