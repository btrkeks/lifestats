use std::any::Any;
use std::sync::Arc;

use crate::error::ApplicationResult;
use chrono::{Duration, Local, NaiveDate, NaiveDateTime};
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::shared::seconds::Seconds;
use lifestats_core::shared::time_frame_list::TimeFrameList;
use lifestats_core::statistic::any_statistic::{AnyGoal, AnyStatistic, StreakDayResult};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::value_objects::goals::amount_goal::AmountGoal;
use lifestats_core::statistic::value_objects::goals::finished_in_time_goal::FinishedInTimeGoal;
use lifestats_core::statistic::value_objects::goals::habit_goal::HabitGoal;
use lifestats_core::statistic::value_objects::goals::time_spans_goal::TimeSpansGoal;
use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

#[derive(Clone)]
pub struct StatisticQueryService<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    statistic_repository: Arc<R>,
}

impl<R> StatisticQueryService<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(statistic_repository: Arc<R>) -> Self {
        Self {
            statistic_repository,
        }
    }

    pub fn get_all_statistics(&self) -> ApplicationResult<Vec<AnyStatistic>> {
        Ok(self.statistic_repository.find_all_metadata()?)
    }

    pub fn get_all_metadata(&self) -> ApplicationResult<Vec<StatisticMetadata>> {
        Ok(self
            .statistic_repository
            .find_all_metadata()?
            .into_iter()
            .map(|stat| stat.get_metadata().clone())
            .collect())
    }

    pub fn get_time_spans_metadata(&self) -> ApplicationResult<Vec<StatisticMetadata>> {
        Ok(self
            .statistic_repository
            .find_all_metadata()?
            .into_iter()
            .filter_map(|stat| {
                if matches!(&stat, AnyStatistic::TimeSpans(_)) {
                    Some(stat.get_metadata().clone())
                } else {
                    None
                }
            })
            .collect())
    }

    pub fn get_metadata(&self, statistic_id: StatisticId) -> ApplicationResult<StatisticMetadata> {
        Ok(self
            .statistic_repository
            .find_metadata_by_id(statistic_id)?
            .get_metadata()
            .clone())
    }

    // -------------------------------------------------------
    // TODO: Those don't quite belong here

    pub fn get_streak(&self, statistic_id: StatisticId) -> ApplicationResult<u32> {
        let statistic = self
            .statistic_repository
            .find_metadata_by_id(statistic_id)?;
        let default_goal_valid_since = statistic.get_default_goal_valid_since();

        let streak = match &statistic {
            AnyStatistic::Habit(_) => self.compute_streak_for::<bool, HabitGoal>(
                statistic_id,
                &statistic,
                statistic.get_default_goal().and_then(|goal| match goal {
                    AnyGoal::Habit(goal) => Some(goal),
                    _ => None,
                }),
                default_goal_valid_since,
            )?,
            AnyStatistic::IntegerAmount(_) => self.compute_streak_for::<i64, AmountGoal<i64>>(
                statistic_id,
                &statistic,
                statistic.get_default_goal().and_then(|goal| match goal {
                    AnyGoal::IntegerAmount(goal) => Some(goal),
                    _ => None,
                }),
                default_goal_valid_since,
            )?,
            AnyStatistic::PointInTime(_) => self
                .compute_streak_for::<NaiveDateTime, FinishedInTimeGoal>(
                    statistic_id,
                    &statistic,
                    statistic.get_default_goal().and_then(|goal| match goal {
                        AnyGoal::FinishedInTime(goal) => Some(goal),
                        _ => None,
                    }),
                    default_goal_valid_since,
                )?,
            AnyStatistic::TimeSpans(_) => self.compute_streak_for::<TimeFrameList, TimeSpansGoal>(
                statistic_id,
                &statistic,
                statistic.get_default_goal().and_then(|goal| match goal {
                    AnyGoal::TimeSpans(goal) => Some(goal),
                    _ => None,
                }),
                default_goal_valid_since,
            )?,
            AnyStatistic::Summing(_) => self.compute_streak_for::<Seconds, AmountGoal<Seconds>>(
                statistic_id,
                &statistic,
                statistic.get_default_goal().and_then(|goal| match goal {
                    AnyGoal::SecondsAmount(goal) => Some(goal),
                    _ => None,
                }),
                default_goal_valid_since,
            )?,
            AnyStatistic::TimeDifference(_) => self
                .compute_streak_for::<Seconds, AmountGoal<Seconds>>(
                    statistic_id,
                    &statistic,
                    statistic.get_default_goal().and_then(|goal| match goal {
                        AnyGoal::SecondsAmount(goal) => Some(goal),
                        _ => None,
                    }),
                    default_goal_valid_since,
                )?,
        };

        Ok(streak)
    }

    fn compute_streak_for<LogValue, GoalType>(
        &self,
        statistic_id: StatisticId,
        statistic: &AnyStatistic,
        default_goal: Option<GoalType>,
        default_goal_valid_since: NaiveDate,
    ) -> anyhow::Result<u32>
    where
        LogValue: serde::de::DeserializeOwned + 'static,
        GoalType: serde::de::DeserializeOwned + Clone + 'static,
    {
        const MAX_LOOKBACK_DAYS: usize = 90;
        let mut streak: u32 = 0;
        let today = Local::now().date_naive();

        for offset in 0..MAX_LOOKBACK_DAYS {
            let date_to_check = today - Duration::days(offset as i64);

            let log_value_json = self
                .statistic_repository
                .find_log_value(statistic_id, date_to_check)?;
            let log_value: Option<LogValue> = log_value_json
                .map(|v| serde_json::from_value(v))
                .transpose()?;

            let goal_json = self
                .statistic_repository
                .find_goal(statistic_id, date_to_check)?;
            let mut goal: Option<GoalType> =
                goal_json.map(|v| serde_json::from_value(v)).transpose()?;

            if goal.is_none() && date_to_check >= default_goal_valid_since {
                goal = default_goal.clone();
            }

            let result = statistic.evaluate_streak_for_day(
                log_value.as_ref().map(|value| value as &dyn Any),
                goal.as_ref().map(|goal| goal as &dyn Any),
            )?;

            match result {
                StreakDayResult::Achieved => streak += 1,
                StreakDayResult::Missed => break,
                StreakDayResult::NoGoalContinue => continue,
            }
        }

        Ok(streak)
    }
}
