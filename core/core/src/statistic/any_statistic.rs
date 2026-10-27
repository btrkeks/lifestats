use crate::shared::seconds::Seconds;
use crate::shared::time_frame_list::TimeFrameList;
use crate::shared::timed_amount_list::TimedAmountList;
use crate::statistic::error::{DomainError, DomainResult};
use crate::statistic::traits::base_statistic::Statistic;
use crate::statistic::traits::integer_statistic::IntegerStatistic;
use crate::statistic::types::derived_statistics::summing_statistic::SummingStatistic;
use crate::statistic::types::derived_statistics::time_difference::TimeDifferenceStatistic;
use crate::statistic::types::logable_statistics::habit::HabitStatistic;
use crate::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
use crate::statistic::types::logable_statistics::point_in_time::PointInTimeStatistic;
use crate::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;
use crate::statistic::types::logable_statistics::timed_amounts::TimedAmountsStatistic;
use crate::statistic::value_objects::goals::amount_goal::AmountGoal;
use crate::statistic::value_objects::goals::finished_in_time_goal::FinishedInTimeGoal;
use crate::statistic::value_objects::goals::goal_trait::Goal;
use crate::statistic::value_objects::goals::habit_goal::HabitGoal;
use crate::statistic::value_objects::goals::time_spans_goal::TimeSpansGoal;
use crate::statistic::value_objects::goals::timed_amounts_goal::TimedAmountsGoal;
use crate::statistic::value_objects::statistic_metadata::StatisticMetadata;
use chrono::{NaiveDate, NaiveTime};
use std::any::{type_name, Any};

fn downcast_option<'a, T: 'static>(
    metadata: &StatisticMetadata,
    input: Option<&'a dyn Any>,
    label: &str,
) -> DomainResult<Option<&'a T>> {
    match input {
        Some(raw) => raw.downcast_ref::<T>().map(Some).ok_or_else(|| {
            DomainError::InvalidOperation(format!(
                "Statistic {} ({}) expected {label} of type {} but received incompatible type",
                metadata.id(),
                metadata.name(),
                type_name::<T>(),
            ))
        }),
        None => Ok(None),
    }
}

fn evaluate_streak_for_day_inner<'a, V, G>(
    metadata: &StatisticMetadata,
    log_value: Option<&'a dyn Any>,
    goal: Option<&'a dyn Any>,
) -> DomainResult<StreakDayResult>
where
    V: 'static,
    G: Goal<LogValue = V> + 'static,
{
    let goal_ref = downcast_option::<G>(metadata, goal, "goal")?;

    if let Some(goal_ref) = goal_ref {
        let value_ref = downcast_option::<V>(metadata, log_value, "log value")?;

        if goal_ref.is_achieved(value_ref) {
            Ok(StreakDayResult::Achieved)
        } else {
            Ok(StreakDayResult::Missed)
        }
    } else {
        Ok(StreakDayResult::NoGoalContinue)
    }
}

/// Type-erased wrapper for goals from any statistic type
#[derive(Debug, Clone, serde::Serialize)]
pub enum AnyGoal {
    Habit(HabitGoal),
    IntegerAmount(AmountGoal<i64>),
    FinishedInTime(FinishedInTimeGoal),
    TimeSpans(TimeSpansGoal),
    SecondsAmount(AmountGoal<Seconds>),
    TimedAmounts(TimedAmountsGoal),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreakDayResult {
    Achieved,
    Missed,
    NoGoalContinue,
}

/// Type-erased statistic wrapper that can hold any statistic type
#[derive(Clone)]
pub enum AnyStatistic {
    Habit(HabitStatistic),
    IntegerAmount(IntegerAmountStatistic),
    PointInTime(PointInTimeStatistic),
    TimeSpans(TimeSpansStatistic),
    TimedAmounts(TimedAmountsStatistic),
    Summing(SummingStatistic),
    TimeDifference(TimeDifferenceStatistic),
}

/// Macro to dispatch method calls to the inner statistic variant
macro_rules! dispatch_statistic {
    ($self:ident, $method:ident, $($args:tt)*) => {
        match $self {
            AnyStatistic::Habit(s) => s.$method($($args)*),
            AnyStatistic::IntegerAmount(s) => s.$method($($args)*),
            AnyStatistic::PointInTime(s) => s.$method($($args)*),
            AnyStatistic::TimeSpans(s) => s.$method($($args)*),
            AnyStatistic::TimedAmounts(s) => s.$method($($args)*),
            AnyStatistic::Summing(s) => s.$method($($args)*),
            AnyStatistic::TimeDifference(s) => s.$method($($args)*),
        }
    };
}

// Inherent methods on AnyStatistic (does NOT implement Statistic trait)
impl AnyStatistic {
    /// Get metadata from the inner statistic
    pub fn get_metadata(&self) -> &StatisticMetadata {
        dispatch_statistic!(self, get_metadata,)
    }

    /// Attempts to convert a statistic's log value (encoded as JSON) into an integer.
    pub fn value_as_i64_from_json(&self, value_json: &serde_json::Value) -> DomainResult<i64> {
        use serde::de::DeserializeOwned;

        fn deserialize<T: DeserializeOwned>(
            value_json: &serde_json::Value,
            metadata: &StatisticMetadata,
            expected: &str,
        ) -> DomainResult<T> {
            serde_json::from_value(value_json.clone()).map_err(|e| {
                DomainError::ValidationError(format!(
                    "Statistic {} ({}) expected {expected} log value but received incompatible JSON: {}",
                    metadata.id(),
                    metadata.name(),
                    e
                ))
            })
        }

        match self {
            AnyStatistic::Habit(s) => {
                let metadata = s.get_metadata();
                let value: bool = deserialize(value_json, metadata, "boolean")?;
                Ok(s.value_as_i64(&value))
            }
            AnyStatistic::IntegerAmount(s) => {
                let metadata = s.get_metadata();
                let value: i64 = deserialize(value_json, metadata, "integer")?;
                Ok(s.value_as_i64(&value))
            }
            AnyStatistic::PointInTime(s) => {
                let metadata = s.get_metadata();
                let value: NaiveTime = deserialize(value_json, metadata, "time")?;
                Ok(s.value_as_i64(&value))
            }
            AnyStatistic::TimeSpans(s) => {
                let metadata = s.get_metadata();
                let value: TimeFrameList = deserialize(value_json, metadata, "time frame list")?;
                Ok(s.value_as_i64(&value))
            }
            AnyStatistic::TimedAmounts(s) => {
                let metadata = s.get_metadata();
                let value: TimedAmountList =
                    deserialize(value_json, metadata, "timed amount list")?;
                Ok(s.value_as_i64(&value))
            }
            AnyStatistic::Summing(s) => {
                let metadata = s.get_metadata();
                let value: Seconds = deserialize(value_json, metadata, "seconds duration")?;
                Ok(s.value_as_i64(&value))
            }
            AnyStatistic::TimeDifference(s) => {
                let metadata = s.get_metadata();
                let value: Seconds = deserialize(value_json, metadata, "seconds duration")?;
                Ok(s.value_as_i64(&value))
            }
        }
    }

    /// Get the statistic ID
    pub fn id(&self) -> crate::shared::ids::StatisticId {
        self.get_metadata().id()
    }

    /// Get the default goal, returning an owned AnyGoal
    pub fn get_default_goal(&self) -> Option<AnyGoal> {
        match self {
            AnyStatistic::Habit(s) => s.get_default_goal().map(|g| AnyGoal::Habit(g.clone())),
            AnyStatistic::IntegerAmount(s) => s
                .get_default_goal()
                .map(|g| AnyGoal::IntegerAmount(g.clone())),
            AnyStatistic::PointInTime(s) => s
                .get_default_goal()
                .map(|g| AnyGoal::FinishedInTime(g.clone())),
            AnyStatistic::TimeSpans(s) => {
                s.get_default_goal().map(|g| AnyGoal::TimeSpans(g.clone()))
            }
            AnyStatistic::TimedAmounts(s) => s
                .get_default_goal()
                .map(|g| AnyGoal::TimedAmounts(g.clone())),
            AnyStatistic::Summing(s) => s
                .get_default_goal()
                .map(|g| AnyGoal::SecondsAmount(g.clone())),
            AnyStatistic::TimeDifference(s) => s
                .get_default_goal()
                .map(|g| AnyGoal::SecondsAmount(g.clone())),
        }
    }

    /// Get the date from which the current default goal became active
    pub fn get_default_goal_valid_since(&self) -> NaiveDate {
        dispatch_statistic!(self, get_default_goal_valid_since,)
    }

    pub fn evaluate_streak_for_day(
        &self,
        log_value: Option<&dyn Any>,
        goal: Option<&dyn Any>,
    ) -> DomainResult<StreakDayResult> {
        let metadata = self.get_metadata();

        match self {
            AnyStatistic::Habit(_) => {
                evaluate_streak_for_day_inner::<bool, HabitGoal>(metadata, log_value, goal)
            }
            AnyStatistic::IntegerAmount(_) => {
                evaluate_streak_for_day_inner::<i64, AmountGoal<i64>>(metadata, log_value, goal)
            }
            AnyStatistic::PointInTime(_) => evaluate_streak_for_day_inner::<
                NaiveTime,
                FinishedInTimeGoal,
            >(metadata, log_value, goal),
            AnyStatistic::TimeSpans(_) => evaluate_streak_for_day_inner::<
                TimeFrameList,
                TimeSpansGoal,
            >(metadata, log_value, goal),
            AnyStatistic::TimedAmounts(_) => evaluate_streak_for_day_inner::<
                TimedAmountList,
                TimedAmountsGoal,
            >(metadata, log_value, goal),
            AnyStatistic::Summing(_) => {
                evaluate_streak_for_day_inner::<Seconds, AmountGoal<Seconds>>(
                    metadata, log_value, goal,
                )
            }
            AnyStatistic::TimeDifference(_) => evaluate_streak_for_day_inner::<
                Seconds,
                AmountGoal<Seconds>,
            >(metadata, log_value, goal),
        }
    }

    //===========================================================================
    // TYPE-SAFE DEFAULT GOAL SETTERS
    //===========================================================================

    /// Set a new default goal for a Habit statistic
    pub fn set_habit_default_goal(
        &mut self,
        goal: HabitGoal,
        start_date: NaiveDate,
    ) -> DomainResult<()> {
        if let AnyStatistic::Habit(s) = self {
            s.set_new_default_goal(goal, start_date)
        } else {
            Err(crate::statistic::error::DomainError::InvalidOperation(
                "Not a Habit statistic".into(),
            ))
        }
    }

    /// Set a new default goal for an IntegerAmount statistic
    pub fn set_integer_amount_default_goal(
        &mut self,
        goal: AmountGoal<i64>,
        start_date: NaiveDate,
    ) -> DomainResult<()> {
        if let AnyStatistic::IntegerAmount(s) = self {
            s.set_new_default_goal(goal, start_date)
        } else {
            Err(crate::statistic::error::DomainError::InvalidOperation(
                "Not an IntegerAmount statistic".into(),
            ))
        }
    }

    /// Set a new default goal for a PointInTime statistic
    pub fn set_point_in_time_default_goal(
        &mut self,
        goal: FinishedInTimeGoal,
        start_date: NaiveDate,
    ) -> DomainResult<()> {
        if let AnyStatistic::PointInTime(s) = self {
            s.set_new_default_goal(goal, start_date)
        } else {
            Err(crate::statistic::error::DomainError::InvalidOperation(
                "Not a PointInTime statistic".into(),
            ))
        }
    }

    /// Set a new default goal for a TimeSpans statistic
    pub fn set_time_spans_default_goal(
        &mut self,
        goal: TimeSpansGoal,
        start_date: NaiveDate,
    ) -> DomainResult<()> {
        if let AnyStatistic::TimeSpans(s) = self {
            s.set_new_default_goal(goal, start_date)
        } else {
            Err(crate::statistic::error::DomainError::InvalidOperation(
                "Not a TimeSpans statistic".into(),
            ))
        }
    }

    /// Set a new default goal for a Summing statistic
    pub fn set_summing_default_goal(
        &mut self,
        goal: AmountGoal<Seconds>,
        start_date: NaiveDate,
    ) -> DomainResult<()> {
        if let AnyStatistic::Summing(s) = self {
            s.set_new_default_goal(goal, start_date)
        } else {
            Err(crate::statistic::error::DomainError::InvalidOperation(
                "Not a Summing statistic".into(),
            ))
        }
    }

    /// Set a new default goal for a TimeDifference statistic
    pub fn set_time_difference_default_goal(
        &mut self,
        goal: AmountGoal<Seconds>,
        start_date: NaiveDate,
    ) -> DomainResult<()> {
        if let AnyStatistic::TimeDifference(s) = self {
            s.set_new_default_goal(goal, start_date)
        } else {
            Err(crate::statistic::error::DomainError::InvalidOperation(
                "Not a TimeDifference statistic".into(),
            ))
        }
    }
}

// Convenience constructors for creating AnyStatistic from concrete types
impl AnyStatistic {
    pub fn from_habit(stat: HabitStatistic) -> Self {
        AnyStatistic::Habit(stat)
    }

    pub fn from_integer_amount(stat: IntegerAmountStatistic) -> Self {
        AnyStatistic::IntegerAmount(stat)
    }

    pub fn from_point_in_time(stat: PointInTimeStatistic) -> Self {
        AnyStatistic::PointInTime(stat)
    }

    pub fn from_time_spans(stat: TimeSpansStatistic) -> Self {
        AnyStatistic::TimeSpans(stat)
    }

    pub fn from_summing(stat: SummingStatistic) -> Self {
        AnyStatistic::Summing(stat)
    }

    pub fn from_time_difference(stat: TimeDifferenceStatistic) -> Self {
        AnyStatistic::TimeDifference(stat)
    }
}

// Downcasting methods for extracting concrete types
impl AnyStatistic {
    pub fn into_habit(self) -> Option<HabitStatistic> {
        match self {
            AnyStatistic::Habit(s) => Some(s),
            _ => None,
        }
    }

    pub fn into_integer_amount(self) -> Option<IntegerAmountStatistic> {
        match self {
            AnyStatistic::IntegerAmount(s) => Some(s),
            _ => None,
        }
    }

    pub fn into_point_in_time(self) -> Option<PointInTimeStatistic> {
        match self {
            AnyStatistic::PointInTime(s) => Some(s),
            _ => None,
        }
    }

    pub fn into_time_spans(self) -> Option<TimeSpansStatistic> {
        match self {
            AnyStatistic::TimeSpans(s) => Some(s),
            _ => None,
        }
    }

    pub fn into_summing(self) -> Option<SummingStatistic> {
        match self {
            AnyStatistic::Summing(s) => Some(s),
            _ => None,
        }
    }

    pub fn into_time_difference(self) -> Option<TimeDifferenceStatistic> {
        match self {
            AnyStatistic::TimeDifference(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_habit(&self) -> Option<&HabitStatistic> {
        match self {
            AnyStatistic::Habit(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_integer_amount(&self) -> Option<&IntegerAmountStatistic> {
        match self {
            AnyStatistic::IntegerAmount(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_point_in_time(&self) -> Option<&PointInTimeStatistic> {
        match self {
            AnyStatistic::PointInTime(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_time_spans(&self) -> Option<&TimeSpansStatistic> {
        match self {
            AnyStatistic::TimeSpans(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_summing(&self) -> Option<&SummingStatistic> {
        match self {
            AnyStatistic::Summing(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_time_difference(&self) -> Option<&TimeDifferenceStatistic> {
        match self {
            AnyStatistic::TimeDifference(s) => Some(s),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {}
