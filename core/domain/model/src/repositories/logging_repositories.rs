use crate::statistic::statistic_types::habit::HabitLogsRepository;
use crate::statistic::statistic_types::integer_amount::IntegerAmountLogsRepository;
use crate::statistic::statistic_types::point_in_time::PointInTimeLogsRepository;
use crate::statistic::statistic_types::time_amount::TimeAmountLogsRepository;
use crate::statistic::statistic_types::time_spans::TimeSpansLogsRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct LoggingRepositories {
    pub habit_repo: Arc<dyn HabitLogsRepository>,
    pub integer_amount_repo: Arc<dyn IntegerAmountLogsRepository>,
    pub point_in_time_repo: Arc<dyn PointInTimeLogsRepository>,
    pub time_amount_repo: Arc<dyn TimeAmountLogsRepository>,
    pub time_spans_repo: Arc<dyn TimeSpansLogsRepository>,
}

impl LoggingRepositories {
    pub fn new(
        habit_repo: Arc<impl HabitLogsRepository + 'static>,
        integer_amount_repo: Arc<impl IntegerAmountLogsRepository + 'static>,
        point_in_time_repo: Arc<impl PointInTimeLogsRepository + 'static>,
        time_amount_repo: Arc<impl TimeAmountLogsRepository + 'static>,
        time_spans_repo: Arc<impl TimeSpansLogsRepository + 'static>,
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
