/// This application provides a cli interface for the user
/// to log new data for existing statistics.
/// The user can also specify an update policy indicating how a data conflict should be handled
// TODO: Allow setting goals from this interface
// TODO: Allow listing all defined statistics with their ids
use anyhow::{Context, Result};
use chrono::{Duration, NaiveDate};
use clap::Parser;
#[cfg(not(test))]
use lifestats_core::app_context::build_application;
use lifestats_core::app_context::AppContext;
use std::sync::Arc;
use testing_utilities::time_provider::{RealTimeProvider, TimeProvider};
use transfer_objects::statistic_id::StatisticId;
use util::stdout_logging::init_logging;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, help = "Id of the statistic")]
    id: u32,

    #[arg(long, help = "What to do in case of existing values")]
    update_policy: Option<String>,

    #[arg(
        short,
        default_value = "0",
        help = "Number of days in the past to log for"
    )]
    days_in_past: i64,

    #[arg(short, long, help = "Value to add")]
    value: Option<String>,

    #[arg(long, help = "Use current time as the value")]
    now: bool,
}

pub struct CliLogger<T: TimeProvider> {
    time_provider: T,
    app_context: AppContext,
}

impl<T: TimeProvider> CliLogger<T> {
    pub fn new(time_provider: T, app_context: AppContext) -> Self {
        Self {
            time_provider,
            app_context,
        }
    }

    pub fn get_date_for_days_in_past(&self, days_in_past: i64) -> NaiveDate {
        self.time_provider.now().date_naive() - Duration::days(days_in_past)
    }

    fn run(&self, args: Args) -> Result<()> {
        let value = if args.now {
            self.time_provider.now().format("%H:%M:%S").to_string()
        } else {
            args.value
                .with_context(|| "Value must be provided if --now is not used")?
        };

        let date = self.get_date_for_days_in_past(args.days_in_past);

        lifestats_core::log::log_statistic(
            &self.app_context,
            StatisticId(args.id),
            date,
            &value,
            args.update_policy.as_deref(),
        )
    }
}

#[cfg(not(test))]
fn main() -> Result<()> {
    init_logging();

    let args = Args::parse();
    let app_context = build_application()?;
    let logger = CliLogger::new(RealTimeProvider, app_context);

    logger.run(args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Local, TimeZone};
    use domain_models::repositories::statistic_repository::MockStatisticRepositoryTrait;
    use domain_models::statistic::statistic_types::any_statistic::AnyStatistic;
    use domain_models::statistic::statistic_types::point_in_time::PointInTimeStatistic;
    use domain_models::statistic::statistic_types::time_spans::TimeSpansStatistic;
    use lifestats_core::testing::mock_app_context_builder::MockAppContextBuilder;
    use lifestats_core::testing::mock_repositories::MockPointInTimeGoalsRepository;
    use lifestats_core::testing::mock_repositories::{
        MockPointInTimeLogsRepository, MockTimeSpansGoalsRepository, MockTimeSpansLogsRepository,
    };
    use mockall::predicate::eq;
    use std::sync::Arc;
    use testing_utilities::time_provider::MockTimeProvider;
    use transfer_objects::statistic_metadata::StatisticMetadata;

    pub fn get_test_date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2020, 10, 10).unwrap()
    }

    fn get_mock_logging_repo() -> MockPointInTimeLogsRepository {
        let mut mock_logging_repo = MockPointInTimeLogsRepository::new();

        mock_logging_repo
            .expect_insert_or_update()
            .returning(|_, _, _| Ok(()));

        mock_logging_repo
            .expect_get()
            .with(eq(StatisticId(20)), eq(get_test_date()))
            .returning(|_, _| Ok(None));

        mock_logging_repo
    }

    fn get_mock_goal_repo() -> MockPointInTimeGoalsRepository {
        let mut mock_goal_repo = MockPointInTimeGoalsRepository::new();
        mock_goal_repo.expect_get().returning(|_, _| Ok(None));
        mock_goal_repo.expect_get_default().returning(|_| Ok(None));
        mock_goal_repo
    }

    fn get_mock_time_spans_logging_repo() -> MockTimeSpansLogsRepository {
        let mut mock_logging_repo = MockTimeSpansLogsRepository::new();

        mock_logging_repo
            .expect_insert_or_update()
            .returning(|_, _, _| Ok(()));

        mock_logging_repo
            .expect_get()
            .with(eq(StatisticId(6)), eq(get_test_date()))
            .returning(|_, _| Ok(None));

        mock_logging_repo
    }

    fn get_mock_time_spans_goal_repo() -> MockTimeSpansGoalsRepository {
        let mut mock_goal_repo = MockTimeSpansGoalsRepository::new();
        mock_goal_repo.expect_get().returning(|_, _| Ok(None));
        mock_goal_repo.expect_get_default().returning(|_| Ok(None));
        mock_goal_repo
    }

    fn setup_mock_context(stat_type: TestStatType) -> AppContext {
        let mut mock_stat_repo = MockStatisticRepositoryTrait::new();

        match stat_type {
            TestStatType::PointInTime => {
                let mock_logging_repo = Arc::new(get_mock_logging_repo());
                let mock_goal_repo = Arc::new(get_mock_goal_repo());

                mock_stat_repo.expect_get_by_id().returning(move |_| {
                    Ok(AnyStatistic::PointInTime(PointInTimeStatistic::new(
                        StatisticMetadata::new(
                            StatisticId(20),
                            "健康".to_string().into_boxed_str(),
                            "就寝時間".to_string().into_boxed_str(),
                        ),
                        mock_logging_repo.clone(),
                        mock_goal_repo.clone(),
                    )))
                });
            }
            TestStatType::TimeSpans => {
                let mock_logging_repo = Arc::new(get_mock_time_spans_logging_repo());
                let mock_goal_repo = Arc::new(get_mock_time_spans_goal_repo());

                mock_stat_repo.expect_get_by_id().returning(move |_| {
                    Ok(AnyStatistic::TimeSpans(TimeSpansStatistic::new(
                        StatisticMetadata::new(
                            StatisticId(6),
                            "仕事".to_string().into_boxed_str(),
                            "集中した時間".to_string().into_boxed_str(),
                        ),
                        mock_logging_repo.clone(),
                        mock_goal_repo.clone(),
                    )))
                });
            }
        }

        MockAppContextBuilder::new()
            .with_statistic_repository(mock_stat_repo)
            .build()
    }

    #[derive(Copy, Clone)]
    enum TestStatType {
        PointInTime,
        TimeSpans,
    }

    fn get_mock_time() -> MockTimeProvider {
        let mut mock_time = MockTimeProvider::new();
        mock_time
    }

    #[test]
    fn test_args_parsing() {
        let args = Args::parse_from([
            "cli_logger",
            "--id",
            "123",
            "--value",
            "test_value",
            "-d",
            "2",
        ]);

        assert_eq!(args.id, 123);
        assert_eq!(args.value, Some("test_value".to_string()));
        assert_eq!(args.days_in_past, 2);
        assert!(!args.now);
    }

    #[test]
    fn test_args_parsing_with_now() {
        let args = Args::parse_from(["cli_logger", "--id", "123", "--now"]);

        assert_eq!(args.id, 123);
        assert!(args.now);
        assert_eq!(args.value, None);
    }

    #[test]
    fn test_get_date_for_days_in_past_1() {
        let mock_time = get_mock_time();
        let test_date = mock_time.now().date_naive();

        let app_context = setup_mock_context(TestStatType::PointInTime);
        let logger = CliLogger::new(mock_time, app_context);

        let result = logger.get_date_for_days_in_past(1);

        assert_eq!(result, test_date - Duration::days(1));
    }

    #[test]
    fn test_get_date_for_days_in_past_2() {
        let mock_time = get_mock_time();
        let test_date = mock_time.now().date_naive();

        let app_context = setup_mock_context(TestStatType::PointInTime);
        let logger = CliLogger::new(mock_time, app_context);

        let result = logger.get_date_for_days_in_past(89);

        assert_eq!(result, test_date - Duration::days(89));
    }

    #[test]
    fn test_can_log_hour_minute() {
        let mock_time = get_mock_time();
        let test_date = mock_time.now().date_naive();

        let app_context = setup_mock_context(TestStatType::PointInTime);
        let logger = CliLogger::new(mock_time, app_context);

        let args = Args {
            id: 20,
            update_policy: None,
            days_in_past: 0,
            value: Some("10:30".to_string()),
            now: false,
        };

        let result = logger.run(args);

        assert!(result.is_ok());
    }

    #[test]
    fn test_can_log_hour_minute_seconds() {
        let mock_time = get_mock_time();
        let app_context = setup_mock_context(TestStatType::PointInTime);
        let logger = CliLogger::new(mock_time, app_context);

        let args = Args {
            id: 20,
            update_policy: None,
            days_in_past: 0,
            value: Some("10:30:11".to_string()),
            now: false,
        };

        let result = logger.run(args);

        assert!(result.is_ok());
    }

    #[test]
    fn test_run_with_now_flag() {
        let mock_time = get_mock_time();
        let app_context = setup_mock_context(TestStatType::PointInTime);
        let logger = CliLogger::new(mock_time, app_context);

        let args = Args {
            id: 20,
            update_policy: None,
            days_in_past: 0,
            value: None,
            now: true,
        };

        let result = logger.run(args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_without_value_and_now_flag() {
        let mock_time = get_mock_time();
        let app_context = setup_mock_context(TestStatType::PointInTime);
        let logger = CliLogger::new(mock_time, app_context);

        let args = Args {
            id: 123,
            update_policy: None,
            days_in_past: 0,
            value: None,
            now: false,
        };

        let result = logger.run(args);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Value must be provided"));
    }

    #[test]
    fn test_can_log_time_spans_hour_minute() {
        let mock_time = get_mock_time();
        let app_context = setup_mock_context(TestStatType::TimeSpans);
        let logger = CliLogger::new(mock_time, app_context);

        let args = Args {
            id: 6,
            update_policy: None,
            days_in_past: 0,
            value: Some("10:30-12:30".to_string()),
            now: false,
        };

        let result = logger.run(args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_can_log_time_spans_with_seconds() {
        let mock_time = get_mock_time();
        let app_context = setup_mock_context(TestStatType::TimeSpans);
        let logger = CliLogger::new(mock_time, app_context);

        let args = Args {
            id: 6,
            update_policy: None,
            days_in_past: 0,
            value: Some("10:33:21-12:59:59".to_string()),
            now: false,
        };

        let result = logger.run(args);
        assert!(result.is_ok());
    }

    // Log Value is a TimeFrame and not a list
    #[ignore]
    #[test]
    fn test_can_log_multiple_time_spans() {
        let mock_time = get_mock_time();
        let app_context = setup_mock_context(TestStatType::TimeSpans);
        let logger = CliLogger::new(mock_time, app_context);

        let args = Args {
            id: 6,
            update_policy: None,
            days_in_past: 0,
            value: Some("10:30-12:30,13:00-15:00".to_string()),
            now: false,
        };

        let result = logger.run(args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_time_spans_format() {
        let mock_time = get_mock_time();
        let app_context = setup_mock_context(TestStatType::TimeSpans);
        let logger = CliLogger::new(mock_time, app_context);

        let args = Args {
            id: 6,
            update_policy: None,
            days_in_past: 0,
            value: Some("invalid format".to_string()),
            now: false,
        };

        let result = logger.run(args);
        assert!(result.is_err());
    }
}
