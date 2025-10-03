use crate::shared::dated_value::DatedValue;
use crate::shared::ids::StatisticId;
use crate::shared::percentage::Percentage;
use crate::statistic::error::Result;
use crate::statistic::value_objects::goals::goal_trait::Goal;
use crate::statistic::value_objects::statistic_metadata::StatisticMetadata;
use chrono::NaiveDate;

pub trait Statistic {
    type LogValueType;
    type GoalType: Goal<LogValue = Self::LogValueType>;

    //===========================================================================
    // METADATA
    //===========================================================================

    fn get_metadata(&self) -> &StatisticMetadata;

    fn id(&self) -> StatisticId {
        self.get_metadata().id()
    }

    fn get_category(&self) -> &str {
        self.get_metadata().category()
    }

    fn get_name(&self) -> &str {
        self.get_metadata().name()
    }

    //===========================================================================
    // LOG RETRIEVAL
    //===========================================================================

    /// Returns the logged value for that day if something has been logged
    fn get_log_value(&self, date: chrono::NaiveDate) -> Result<Option<Self::LogValueType>>;

    fn get_log_values_closed_range(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Vec<DatedValue<Self::LogValueType>> {
        let mut result = Vec::new();
        let mut current_date = start;

        while current_date <= end {
            if let Ok(Some(value)) = self.get_log_value(current_date) {
                result.push(DatedValue::new(current_date, value));
            }
            current_date = current_date.succ_opt().unwrap_or(current_date);
            if current_date == start {
                break;
            } // Prevent infinite loop
        }

        result
    }

    //===========================================================================
    // GOAL MANAGEMENT AND EVALUATION
    //===========================================================================

    /// Return the goal for that day if there is a goal set.
    fn get_goal(&self, date: NaiveDate) -> Result<Option<Self::GoalType>>;

    fn set_goal(&self, date: NaiveDate, goal: Self::GoalType) -> Result<()>;

    fn get_goals_closed_range(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Vec<DatedValue<Option<Self::GoalType>>> {
        let mut result = Vec::new();
        let mut current_date = start;

        while current_date <= end {
            let goal = self.get_goal(current_date).unwrap_or(None);
            result.push(DatedValue::new(current_date, goal));
            current_date = current_date.succ_opt().unwrap_or(current_date);
            if current_date == start {
                break;
            } // Prevent infinite loop
        }

        result
    }

    /// Returns if the goal has been achieved on the given date if a goal was set
    /// Returns None if there was no goal set for that day
    fn goal_achieved_on(&self, date: NaiveDate) -> Result<Option<bool>> {
        let goal = self.get_goal(date)?;

        match goal {
            Some(goal) => {
                let log_value = self.get_log_value(date)?;
                Ok(Some(goal.is_achieved(log_value.as_ref())))
            }
            None => Ok(None),
        }
    }

    fn get_score(&self, date: NaiveDate) -> Result<Option<Percentage>> {
        let goal = self.get_goal(date)?;

        match goal {
            Some(goal) => {
                let log_value = self.get_log_value(date)?;
                Ok(Some(goal.get_percentage_achieved(log_value.as_ref())))
            }
            None => Ok(None),
        }
    }

    fn get_scores_closed_range(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<DatedValue<Percentage>>> {
        let mut result = Vec::new();
        let mut current_date = start;

        while current_date <= end {
            if let Some(score) = self.get_score(current_date)? {
                result.push(DatedValue::new(current_date, score));
            }
            current_date = current_date.succ_opt().unwrap_or(current_date);
            if current_date == start {
                break;
            } // Prevent infinite loop
        }

        Ok(result)
    }

    fn get_streak(&self) -> Result<u32> {
        const MAX_NO_GOALS: u32 = 20;

        let mut current_date = chrono::Local::now().date_naive();
        let mut streak = 0;
        let mut num_consecutive_no_goal = 0;
        loop {
            match self.goal_achieved_on(current_date)? {
                Some(true) => streak += 1,
                None if num_consecutive_no_goal < MAX_NO_GOALS => num_consecutive_no_goal += 1,
                _ => break,
            }
            current_date = current_date.pred_opt().unwrap();
        }
        Ok(streak)
    }

    //===========================================================================
    // DEFAULT GOAL MANAGEMENT
    //===========================================================================

    /// Returns the current default goal
    fn get_default_goal(&self) -> Option<&Self::GoalType>;

    /// Sets a new default goal `goal` starting at `start_date` (inclusive).
    /// This function shouldn't be used by user code as it isn't supposed to flush the previous goal
    fn _set_default_goal(&mut self, goal: Self::GoalType, start_date: NaiveDate);

    /// Updates the default goal with `goal` starting from `date`
    /// and flushes the old default goal for previous dates
    fn set_new_default_goal(&mut self, goal: Self::GoalType, start_date: NaiveDate) -> Result<()> {
        // TODO: date might be in the future. We should test for that case

        // Flush the old goal for each date from flush_until to the day before new_goal_start_
        if let Some(current_goal) = self.get_default_goal() {
            let flush_until = self.get_default_goal_valid_since();
            let mut current_date = flush_until;
            while current_date < start_date {
                // Set the goal for this specific date
                self.set_goal(current_date, (*current_goal).clone())?;

                // Move to the next date
                current_date = match current_date.succ_opt() {
                    Some(next_date) => next_date,
                    None => break,
                };
            }
        }

        self._set_default_goal(goal, start_date);
        Ok(())
    }

    /// Returns the date from which the current default goal became active (including)
    fn get_default_goal_valid_since(&self) -> NaiveDate;
}
