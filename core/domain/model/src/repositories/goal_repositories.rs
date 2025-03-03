use crate::statistic::statistic_types::habit::HabitGoalsRepository;
use crate::statistic::statistic_types::integer_amount::IntegerAmountGoalsRepository;
use crate::statistic::statistic_types::point_in_time::PointInTimeGoalsRepository;
use crate::statistic::statistic_types::time_amount::TimeAmountGoalsRepository;
use crate::statistic::statistic_types::time_spans::TimeSpansGoalsRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct GoalRepositories {
    pub habit_repo: Arc<dyn HabitGoalsRepository>,
    pub integer_amount_repo: Arc<dyn IntegerAmountGoalsRepository>,
    pub point_in_time_repo: Arc<dyn PointInTimeGoalsRepository>,
    pub time_amount_repo: Arc<dyn TimeAmountGoalsRepository>,
    pub time_spans_repo: Arc<dyn TimeSpansGoalsRepository>,
}

impl GoalRepositories {
    pub fn new(
        habit_repo: Arc<dyn HabitGoalsRepository>,
        integer_amount_repo: Arc<dyn IntegerAmountGoalsRepository>,
        point_in_time_repo: Arc<dyn PointInTimeGoalsRepository>,
        time_amount_repo: Arc<dyn TimeAmountGoalsRepository>,
        time_spans_repo: Arc<dyn TimeSpansGoalsRepository>,
    ) -> Self {
        Self {
            habit_repo,
            integer_amount_repo,
            point_in_time_repo,
            time_amount_repo,
            time_spans_repo,
        }
    }
}
