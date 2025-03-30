use crate::statistic::error::Result;
use chrono::NaiveDate;

pub trait GoalRepository {
    type GoalType;
    fn get(&self, date: NaiveDate) -> Result<Option<Self::GoalType>>;
    fn set(&self, date: NaiveDate, value: Self::GoalType) -> Result<()>;
}