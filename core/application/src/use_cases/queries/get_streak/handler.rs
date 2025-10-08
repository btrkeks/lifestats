use std::any::Any;
use std::sync::Arc;

use crate::cqrs::QueryHandler;
use crate::error::ApplicationResult;
use chrono::{Duration, Local, NaiveDate};
use lifestats_core::shared::seconds::Seconds;
use lifestats_core::shared::time_frame_list::TimeFrameList;
use lifestats_core::statistic::any_statistic::{AnyGoal, AnyStatistic, StreakDayResult};
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;
use lifestats_core::statistic::value_objects::goals::amount_goal::AmountGoal;
use lifestats_core::statistic::value_objects::goals::finished_in_time_goal::FinishedInTimeGoal;
use lifestats_core::statistic::value_objects::goals::habit_goal::HabitGoal;
use lifestats_core::statistic::value_objects::goals::time_spans_goal::TimeSpansGoal;

use super::query::GetStreakQuery;

/// Handler for GetStreakQuery
///
/// This handler contains complex business logic for streak computation:
/// - Iterates backwards through dates from today
/// - Evaluates goal achievement for each day
/// - Counts consecutive days where goals were met
/// - Handles default goals vs daily goal overrides
pub struct GetStreakQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    repository: Arc<R>,
}

impl<R> GetStreakQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    /// Generic helper to compute streak for any statistic type
    fn compute_streak_for<LogValue, GoalType>(
        &self,
        statistic_id: lifestats_core::shared::ids::StatisticId,
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
                .repository
                .find_log_value(statistic_id, date_to_check)?;
            let log_value: Option<LogValue> = log_value_json
                .map(|v| serde_json::from_value(v))
                .transpose()?;

            let goal_json = self.repository.find_goal(statistic_id, date_to_check)?;
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

impl<R> QueryHandler<GetStreakQuery> for GetStreakQueryHandler<R>
where
    R: StatisticRepository + Send + Sync + 'static,
{
    fn handle(&self, query: GetStreakQuery) -> ApplicationResult<u32> {
        let statistic = self.repository.find_metadata_by_id(query.statistic_id)?;
        let default_goal_valid_since = statistic.get_default_goal_valid_since();

        let streak = match &statistic {
            AnyStatistic::Habit(_) => self.compute_streak_for::<bool, HabitGoal>(
                query.statistic_id,
                &statistic,
                statistic.get_default_goal().and_then(|goal| match goal {
                    AnyGoal::Habit(goal) => Some(goal),
                    _ => None,
                }),
                default_goal_valid_since,
            )?,
            AnyStatistic::IntegerAmount(_) => self.compute_streak_for::<i64, AmountGoal<i64>>(
                query.statistic_id,
                &statistic,
                statistic.get_default_goal().and_then(|goal| match goal {
                    AnyGoal::IntegerAmount(goal) => Some(goal),
                    _ => None,
                }),
                default_goal_valid_since,
            )?,
            AnyStatistic::PointInTime(_) => self
                .compute_streak_for::<chrono::NaiveDateTime, FinishedInTimeGoal>(
                    query.statistic_id,
                    &statistic,
                    statistic.get_default_goal().and_then(|goal| match goal {
                        AnyGoal::FinishedInTime(goal) => Some(goal),
                        _ => None,
                    }),
                    default_goal_valid_since,
                )?,
            AnyStatistic::TimeSpans(_) => self.compute_streak_for::<TimeFrameList, TimeSpansGoal>(
                query.statistic_id,
                &statistic,
                statistic.get_default_goal().and_then(|goal| match goal {
                    AnyGoal::TimeSpans(goal) => Some(goal),
                    _ => None,
                }),
                default_goal_valid_since,
            )?,
            AnyStatistic::Summing(_) => self.compute_streak_for::<Seconds, AmountGoal<Seconds>>(
                query.statistic_id,
                &statistic,
                statistic.get_default_goal().and_then(|goal| match goal {
                    AnyGoal::SecondsAmount(goal) => Some(goal),
                    _ => None,
                }),
                default_goal_valid_since,
            )?,
            AnyStatistic::TimeDifference(_) => self
                .compute_streak_for::<Seconds, AmountGoal<Seconds>>(
                    query.statistic_id,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use lifestats_core::shared::ids::StatisticId;
    use lifestats_core::statistic::ports::statistic_repository::MockStatisticRepository;
    use lifestats_core::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
    use lifestats_core::statistic::value_objects::statistic_metadata::StatisticMetadata;

    #[test]
    fn test_get_streak_handler_compiles_and_runs() {
        // This is a smoke test to verify the handler compiles and can execute
        // The actual streak computation logic is complex and tested in the domain layer

        let mut repo = MockStatisticRepository::new();
        let stat_id = StatisticId(1);
        let today = Local::now().date_naive();

        // Create statistic without default goal (streak will be 0)
        let stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            StatisticMetadata::new(stat_id, "test".into(), "test_stat".into()),
            None, // No default goal
            today,
        ));

        repo.expect_find_metadata_by_id()
            .times(1)
            .return_once(move |_| Ok(stat));

        // Mock repository methods (will return NoGoalContinue for all days)
        repo.expect_find_log_value().returning(|_, _| Ok(None));
        repo.expect_find_goal().returning(|_, _| Ok(None));

        let handler = GetStreakQueryHandler::new(Arc::new(repo));
        let query = GetStreakQuery {
            statistic_id: stat_id,
        };

        // Act
        let result = handler.handle(query);

        // Assert - just verify it doesn't error
        assert!(result.is_ok(), "Handler should execute without error");
        // Streak will be 0 since there's no goal
        assert_eq!(result.unwrap(), 0);
    }
}
