use crate::shared::seconds::Seconds;
use crate::shared::time_frame_list::TimeFrameList;
use crate::statistic::error::Result;
use crate::statistic::traits::base_statistic::Statistic;
use crate::statistic::types::derived_statistics::summing_statistic::SummingStatistic;
use crate::statistic::types::derived_statistics::time_difference::TimeDifferenceStatistic;
use crate::statistic::types::logable_statistics::habit::HabitStatistic;
use crate::statistic::types::logable_statistics::integer_amount::IntegerAmountStatistic;
use crate::statistic::types::logable_statistics::point_in_time::PointInTimeStatistic;
use crate::statistic::types::logable_statistics::time_spans::TimeSpansStatistic;
use crate::statistic::value_objects::goals::amount_goal::AmountGoal;
use crate::statistic::value_objects::goals::finished_in_time_goal::FinishedInTimeGoal;
use crate::statistic::value_objects::goals::habit_goal::HabitGoal;
use crate::statistic::value_objects::goals::time_spans_goal::TimeSpansGoal;
use crate::statistic::value_objects::statistic_metadata::StatisticMetadata;
use chrono::{NaiveDate, NaiveDateTime};

/// Type-erased wrapper for log values from any statistic type
#[derive(Debug, Clone)]
pub enum AnyLogValue {
    Bool(bool),
    Integer(i64),
    DateTime(NaiveDateTime),
    TimeFrames(TimeFrameList),
    Seconds(Seconds),
}

/// Type-erased wrapper for goals from any statistic type
#[derive(Debug, Clone)]
pub enum AnyGoal {
    Habit(HabitGoal),
    IntegerAmount(AmountGoal<i64>),
    FinishedInTime(FinishedInTimeGoal),
    TimeSpans(TimeSpansGoal),
    SecondsAmount(AmountGoal<Seconds>),
}

/// Type-erased statistic wrapper that can hold any statistic type
pub enum AnyStatistic {
    Habit(HabitStatistic),
    IntegerAmount(IntegerAmountStatistic),
    PointInTime(PointInTimeStatistic),
    TimeSpans(TimeSpansStatistic),
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

    /// Get the statistic ID
    pub fn id(&self) -> crate::shared::ids::StatisticId {
        self.get_metadata().id()
    }

    /// Get the log value for a specific date, wrapped in AnyLogValue
    pub fn get_log_value(&self, date: NaiveDate) -> Result<Option<AnyLogValue>> {
        match self {
            AnyStatistic::Habit(s) => s.get_log_value(date).map(|opt| opt.map(AnyLogValue::Bool)),
            AnyStatistic::IntegerAmount(s) => {
                s.get_log_value(date).map(|opt| opt.map(AnyLogValue::Integer))
            }
            AnyStatistic::PointInTime(s) => {
                s.get_log_value(date).map(|opt| opt.map(AnyLogValue::DateTime))
            }
            AnyStatistic::TimeSpans(s) => {
                s.get_log_value(date).map(|opt| opt.map(AnyLogValue::TimeFrames))
            }
            AnyStatistic::Summing(s) => {
                s.get_log_value(date).map(|opt| opt.map(AnyLogValue::Seconds))
            }
            AnyStatistic::TimeDifference(s) => {
                s.get_log_value(date).map(|opt| opt.map(AnyLogValue::Seconds))
            }
        }
    }

    /// Get the goal for a specific date, wrapped in AnyGoal
    pub fn get_goal(&self, date: NaiveDate) -> Result<Option<AnyGoal>> {
        match self {
            AnyStatistic::Habit(s) => s.get_goal(date).map(|opt| opt.map(AnyGoal::Habit)),
            AnyStatistic::IntegerAmount(s) => {
                s.get_goal(date).map(|opt| opt.map(AnyGoal::IntegerAmount))
            }
            AnyStatistic::PointInTime(s) => {
                s.get_goal(date).map(|opt| opt.map(AnyGoal::FinishedInTime))
            }
            AnyStatistic::TimeSpans(s) => {
                s.get_goal(date).map(|opt| opt.map(AnyGoal::TimeSpans))
            }
            AnyStatistic::Summing(s) => {
                s.get_goal(date).map(|opt| opt.map(AnyGoal::SecondsAmount))
            }
            AnyStatistic::TimeDifference(s) => {
                s.get_goal(date).map(|opt| opt.map(AnyGoal::SecondsAmount))
            }
        }
    }

    /// Get the default goal, returning an owned AnyGoal
    pub fn get_default_goal(&self) -> Option<AnyGoal> {
        match self {
            AnyStatistic::Habit(s) => s.get_default_goal().map(|g| AnyGoal::Habit(g.clone())),
            AnyStatistic::IntegerAmount(s) => {
                s.get_default_goal().map(|g| AnyGoal::IntegerAmount(g.clone()))
            }
            AnyStatistic::PointInTime(s) => {
                s.get_default_goal().map(|g| AnyGoal::FinishedInTime(g.clone()))
            }
            AnyStatistic::TimeSpans(s) => {
                s.get_default_goal().map(|g| AnyGoal::TimeSpans(g.clone()))
            }
            AnyStatistic::Summing(s) => {
                s.get_default_goal().map(|g| AnyGoal::SecondsAmount(g.clone()))
            }
            AnyStatistic::TimeDifference(s) => {
                s.get_default_goal().map(|g| AnyGoal::SecondsAmount(g.clone()))
            }
        }
    }

    /// Get the date from which the current default goal became active
    pub fn get_default_goal_valid_since(&self) -> NaiveDate {
        dispatch_statistic!(self, get_default_goal_valid_since,)
    }

    //===========================================================================
    // TYPE-SAFE GOAL SETTERS
    //===========================================================================

    /// Set a goal for a Habit statistic
    pub fn set_habit_goal(&self, date: NaiveDate, goal: HabitGoal) -> Result<()> {
        if let AnyStatistic::Habit(s) = self {
            s.set_goal(date, goal)
        } else {
            Err(crate::statistic::error::StatisticError::InvalidOperation(
                "Not a Habit statistic".into(),
            ))
        }
    }

    /// Set a goal for an IntegerAmount statistic
    pub fn set_integer_amount_goal(&self, date: NaiveDate, goal: AmountGoal<i64>) -> Result<()> {
        if let AnyStatistic::IntegerAmount(s) = self {
            s.set_goal(date, goal)
        } else {
            Err(crate::statistic::error::StatisticError::InvalidOperation(
                "Not an IntegerAmount statistic".into(),
            ))
        }
    }

    /// Set a goal for a PointInTime statistic
    pub fn set_point_in_time_goal(&self, date: NaiveDate, goal: FinishedInTimeGoal) -> Result<()> {
        if let AnyStatistic::PointInTime(s) = self {
            s.set_goal(date, goal)
        } else {
            Err(crate::statistic::error::StatisticError::InvalidOperation(
                "Not a PointInTime statistic".into(),
            ))
        }
    }

    /// Set a goal for a TimeSpans statistic
    pub fn set_time_spans_goal(&self, date: NaiveDate, goal: TimeSpansGoal) -> Result<()> {
        if let AnyStatistic::TimeSpans(s) = self {
            s.set_goal(date, goal)
        } else {
            Err(crate::statistic::error::StatisticError::InvalidOperation(
                "Not a TimeSpans statistic".into(),
            ))
        }
    }

    /// Set a goal for a Summing statistic
    pub fn set_summing_goal(&self, date: NaiveDate, goal: AmountGoal<Seconds>) -> Result<()> {
        if let AnyStatistic::Summing(s) = self {
            s.set_goal(date, goal)
        } else {
            Err(crate::statistic::error::StatisticError::InvalidOperation(
                "Not a Summing statistic".into(),
            ))
        }
    }

    /// Set a goal for a TimeDifference statistic
    pub fn set_time_difference_goal(&self, date: NaiveDate, goal: AmountGoal<Seconds>) -> Result<()> {
        if let AnyStatistic::TimeDifference(s) = self {
            s.set_goal(date, goal)
        } else {
            Err(crate::statistic::error::StatisticError::InvalidOperation(
                "Not a TimeDifference statistic".into(),
            ))
        }
    }

    //===========================================================================
    // TYPE-SAFE DEFAULT GOAL SETTERS
    //===========================================================================

    /// Set a new default goal for a Habit statistic
    pub fn set_habit_default_goal(&mut self, goal: HabitGoal, start_date: NaiveDate) -> Result<()> {
        if let AnyStatistic::Habit(s) = self {
            s.set_new_default_goal(goal, start_date)
        } else {
            Err(crate::statistic::error::StatisticError::InvalidOperation(
                "Not a Habit statistic".into(),
            ))
        }
    }

    /// Set a new default goal for an IntegerAmount statistic
    pub fn set_integer_amount_default_goal(&mut self, goal: AmountGoal<i64>, start_date: NaiveDate) -> Result<()> {
        if let AnyStatistic::IntegerAmount(s) = self {
            s.set_new_default_goal(goal, start_date)
        } else {
            Err(crate::statistic::error::StatisticError::InvalidOperation(
                "Not an IntegerAmount statistic".into(),
            ))
        }
    }

    /// Set a new default goal for a PointInTime statistic
    pub fn set_point_in_time_default_goal(&mut self, goal: FinishedInTimeGoal, start_date: NaiveDate) -> Result<()> {
        if let AnyStatistic::PointInTime(s) = self {
            s.set_new_default_goal(goal, start_date)
        } else {
            Err(crate::statistic::error::StatisticError::InvalidOperation(
                "Not a PointInTime statistic".into(),
            ))
        }
    }

    /// Set a new default goal for a TimeSpans statistic
    pub fn set_time_spans_default_goal(&mut self, goal: TimeSpansGoal, start_date: NaiveDate) -> Result<()> {
        if let AnyStatistic::TimeSpans(s) = self {
            s.set_new_default_goal(goal, start_date)
        } else {
            Err(crate::statistic::error::StatisticError::InvalidOperation(
                "Not a TimeSpans statistic".into(),
            ))
        }
    }

    /// Set a new default goal for a Summing statistic
    pub fn set_summing_default_goal(&mut self, goal: AmountGoal<Seconds>, start_date: NaiveDate) -> Result<()> {
        if let AnyStatistic::Summing(s) = self {
            s.set_new_default_goal(goal, start_date)
        } else {
            Err(crate::statistic::error::StatisticError::InvalidOperation(
                "Not a Summing statistic".into(),
            ))
        }
    }

    /// Set a new default goal for a TimeDifference statistic
    pub fn set_time_difference_default_goal(&mut self, goal: AmountGoal<Seconds>, start_date: NaiveDate) -> Result<()> {
        if let AnyStatistic::TimeDifference(s) = self {
            s.set_new_default_goal(goal, start_date)
        } else {
            Err(crate::statistic::error::StatisticError::InvalidOperation(
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
