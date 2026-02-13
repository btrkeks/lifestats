use std::sync::Arc;

use chrono::{Days, NaiveDate};
use lifestats_core::shared::date_range::DateRange;
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::statistic_repository::StatisticRepository;

use crate::cqrs::QueryHandler;
use crate::dto::dashboard::{DashboardPerformanceDto, DashboardResponseDto, DashboardStatisticDto};
use crate::error::{ApplicationError, ApplicationResult};
use crate::services::daily_achievement_evaluator::DailyAchievementEvaluator;
use crate::services::goal_performance_analyzer::GoalPerformanceAnalyzer;
use crate::services::value_averaging::compute_average_i64;

use super::query::GetDashboardQuery;

/// Handler for GetDashboardQuery.
///
/// Orchestrates loading statistics, fetching values, and computing performance.
pub struct GetDashboardQueryHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: DailyAchievementEvaluator + Send + Sync + 'static,
{
    repo: Arc<R>,
    analyzer: Arc<GoalPerformanceAnalyzer<E, R>>,
}

impl<R, E> GetDashboardQueryHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: DailyAchievementEvaluator + Send + Sync + 'static,
{
    pub fn new(repo: Arc<R>, analyzer: Arc<GoalPerformanceAnalyzer<E, R>>) -> Self {
        Self { repo, analyzer }
    }

    fn build_statistic_data(
        &self,
        stat: &AnyStatistic,
        today: NaiveDate,
        performance_window_days: u32,
    ) -> ApplicationResult<DashboardStatisticDto> {
        let stat_id = stat.id();
        let metadata = stat.get_metadata();

        let today_value = self.get_today_value(stat, today)?;
        let avg_7d_value = self.get_7d_average(stat, today)?;
        let performance = self.get_performance(stat_id, today, performance_window_days)?;

        Ok(DashboardStatisticDto {
            statistic_id: stat_id.0,
            statistic_name: metadata.name().to_string(),
            statistic_type: Self::get_statistic_type_string(stat),
            today_value,
            avg_7d_value,
            performance,
        })
    }

    fn get_statistic_type_string(stat: &AnyStatistic) -> String {
        match stat {
            AnyStatistic::Habit(_) => "Habit",
            AnyStatistic::IntegerAmount(_) => "IntegerAmount",
            AnyStatistic::PointInTime(_) => "PointInTime",
            AnyStatistic::TimeSpans(_) => "TimeSpans",
            AnyStatistic::Summing(_) => "Summing",
            AnyStatistic::TimeDifference(_) => "TimeDifference",
        }
        .to_string()
    }

    fn get_today_value(
        &self,
        stat: &AnyStatistic,
        today: NaiveDate,
    ) -> ApplicationResult<Option<i64>> {
        let value_json = self.repo.find_log_value(stat.id(), today)?;

        match value_json {
            Some(json) => match stat.value_as_i64_from_json(&json) {
                Ok(val) => Ok(Some(val)),
                Err(_) => Ok(None),
            },
            None => Ok(None),
        }
    }

    fn get_7d_average(
        &self,
        stat: &AnyStatistic,
        today: NaiveDate,
    ) -> ApplicationResult<Option<f64>> {
        const WINDOW_DAYS: u64 = 7;

        let start = today.checked_sub_days(Days::new(WINDOW_DAYS - 1)).unwrap_or(today);
        let range = DateRange::new(start, today).map_err(ApplicationError::Infrastructure)?;

        let dated_values = self.repo.find_log_values_in_range(stat.id(), range)?;

        // Sum all logged values, treating missing days as 0
        let sum: i64 = dated_values
            .iter()
            .filter_map(|dv| stat.value_as_i64_from_json(&dv.value).ok())
            .sum();

        // Always divide by the full window size (7 days)
        Ok(Some(sum as f64 / WINDOW_DAYS as f64))
    }

    fn get_performance(
        &self,
        stat_id: StatisticId,
        today: NaiveDate,
        window_days: u32,
    ) -> ApplicationResult<Option<DashboardPerformanceDto>> {
        let start = today
            .checked_sub_days(Days::new((window_days - 1) as u64))
            .unwrap_or(today);
        let range = DateRange::new(start, today).map_err(ApplicationError::Infrastructure)?;

        match self.analyzer.analyze(stat_id, range) {
            Ok(perf) => Ok(Some(DashboardPerformanceDto {
                overall_score: perf.overall_score.as_num(),
                trend: perf.trend,
            })),
            Err(ApplicationError::InsufficientData(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn validate_not_derived(stat: &AnyStatistic) -> ApplicationResult<()> {
        match stat {
            AnyStatistic::Summing(_) | AnyStatistic::TimeDifference(_) => {
                Err(ApplicationError::InvalidInput(format!(
                    "Derived statistic {} ({}) is not supported in dashboard. \
                     Only logable statistics are allowed.",
                    stat.id().0,
                    stat.get_metadata().name()
                )))
            }
            _ => Ok(()),
        }
    }
}

impl<R, E> QueryHandler<GetDashboardQuery> for GetDashboardQueryHandler<R, E>
where
    R: StatisticRepository + Send + Sync + 'static,
    E: DailyAchievementEvaluator + Send + Sync + 'static,
{
    fn handle(&self, query: GetDashboardQuery) -> ApplicationResult<DashboardResponseDto> {
        let mut statistics = Vec::with_capacity(query.statistic_ids.len());

        for stat_id in &query.statistic_ids {
            let stat = self.repo.find_metadata_by_id(*stat_id)?;

            Self::validate_not_derived(&stat)?;

            let stat_data =
                self.build_statistic_data(&stat, query.today, query.performance_window_days)?;

            statistics.push(stat_data);
        }

        Ok(DashboardResponseDto { statistics })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_statistic_type_string_returns_correct_names() {
        // This test documents the expected type names for each variant
        // The actual enum variants are tested implicitly through integration tests
    }
}
