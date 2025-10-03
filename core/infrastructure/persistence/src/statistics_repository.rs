// --- File: /home/daniel/.local/share/src/lifestats/persistence/src/statistics_repository.rs ---
use crate::hydration::statistic_dto::{StatisticDto, StatisticTypeDto};
use crate::sqlite_database::schema::statistics;
use anyhow::{Context, Result};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use lifestats_domain::shared::ids::StatisticId;
use lifestats_domain::statistic::ports::goal_repositories::GoalRepositories;
use lifestats_domain::statistic::ports::logging_repositories::LoggingRepositories;
use lifestats_domain::statistic::ports::statistic_repository::StatisticRepositoryTrait;
use lifestats_domain::statistic::statistic_types::any_statistic::AnyStatistic;
use lifestats_domain::statistic::statistic_types::time_spans::TimeSpansStatistic;
use std::sync::Arc;

pub struct StatisticRepository {
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    logging_repositories: LoggingRepositories,
    goal_repositories: GoalRepositories,
}

impl StatisticRepository {
    pub fn new(
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
        logging_repositories: LoggingRepositories,
        goal_repositories: GoalRepositories,
    ) -> Self {
        Self {
            pool,
            logging_repositories,
            goal_repositories,
        }
    }

    fn get_connection(&self) -> Result<PooledConnection<ConnectionManager<SqliteConnection>>> {
        self.pool.get().context("Failed to get database connection")
    }
}

impl StatisticRepositoryTrait for StatisticRepository {
    fn get_by_id(&self, id: StatisticId) -> Result<AnyStatistic> {
        let mut conn = self.get_connection()?;

        statistics::table
            .find(id.0 as i32)
            .first::<StatisticDto>(&mut conn)
            .map(|raw| raw.into_any_statistic(&self.logging_repositories, &self.goal_repositories))
            .with_context(|| format!("Failed to get statistic with id: {}", id.0))
    }

    fn save(&self, statistic: AnyStatistic) -> Result<()> {
        let mut conn = self.get_connection()?;

        let raw = StatisticDto::from_any_statistic(&statistic);

        diesel::insert_into(statistics::table)
            .values(&raw)
            .on_conflict(statistics::id)
            .do_update()
            .set(&raw)
            .execute(&mut conn)
            .with_context(|| {
                format!(
                    "Failed to save statistic with name: {}",
                    statistic.get_name()
                )
            })?;
        Ok(())
    }

    fn get_all(&self) -> Result<Box<[AnyStatistic]>> {
        let mut conn = self.get_connection()?;

        Ok(statistics::table
            .load::<StatisticDto>(&mut conn)
            .with_context(|| "Failed to get all statistics")?
            .into_iter()
            .map(|raw| raw.into_any_statistic(&self.logging_repositories, &self.goal_repositories))
            .collect())
    }

    fn get_all_time_spans_statistics(&self) -> Result<Box<[TimeSpansStatistic]>> {
        let mut conn = self.get_connection()?;

        statistics::table
            .filter(statistics::statistic_type.eq(StatisticTypeDto::TimeSpans))
            .load::<StatisticDto>(&mut conn)
            .context("Failed to query TimeSpans statistics")
            .map(|loaded| {
                loaded
                    .into_iter()
                    .map(|raw| {
                        raw.into_any_statistic(&self.logging_repositories, &self.goal_repositories)
                            .into_time_spans()
                            .expect("Failed to convert to TimeSpansStatistic")
                    })
                    .collect()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal_repository::GoalRepositoryTemplate;
    use crate::logging_data_repository::LoggingDataRepository;
    use crate::pool::MIGRATIONS;
    use diesel_migrations::MigrationHarness;
    use lifestats_domain::statistic::ports::goal_repositories::GoalRepositories;
    use lifestats_domain::statistic::ports::logging_repositories::LoggingRepositories;
    use lifestats_domain::statistic::statistic::Statistic;
    use lifestats_domain::statistic::statistic_types::habit::HabitStatistic;
    use lifestats_domain::statistic::statistic_types::integer_amount::IntegerAmountStatistic;
    use lifestats_domain::statistic::statistic_types::time_amount::TimeAmountStatistic;
    use lifestats_domain::statistic::value_objects::statistic_metadata::StatisticMetadata;

    // Helper to create dummy repositories
    fn create_repositories(
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    ) -> (LoggingRepositories, GoalRepositories) {
        let logging_repositories = LoggingRepositories::new(
            Arc::new(LoggingDataRepository::new(Arc::clone(&pool))),
            Arc::new(LoggingDataRepository::new(Arc::clone(&pool))),
            Arc::new(LoggingDataRepository::new(Arc::clone(&pool))),
            Arc::new(LoggingDataRepository::new(Arc::clone(&pool))),
            Arc::new(LoggingDataRepository::new(Arc::clone(&pool))),
        );

        let goal_repositories = GoalRepositories::new(
            Arc::new(GoalRepositoryTemplate::new(Arc::clone(&pool))),
            Arc::new(GoalRepositoryTemplate::new(Arc::clone(&pool))),
            Arc::new(GoalRepositoryTemplate::new(Arc::clone(&pool))),
            Arc::new(GoalRepositoryTemplate::new(Arc::clone(&pool))),
            Arc::new(GoalRepositoryTemplate::new(Arc::clone(&pool))),
        );

        (logging_repositories, goal_repositories)
    }

    fn setup_test_db() -> Arc<Pool<ConnectionManager<SqliteConnection>>> {
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder()
            .build(manager)
            .expect("Failed to create pool");
        let mut conn = pool.get().expect("Failed to get connection from pool");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");
        Arc::new(pool)
    }

    #[test]
    fn test_save_and_get_integer_amount_statistic() {
        let pool = setup_test_db();
        let (logging_repos, goal_repos) = create_repositories(Arc::clone(&pool));
        let repo = StatisticRepository::new(pool, logging_repos, goal_repos);

        let metadata = StatisticMetadata::new(
            StatisticId(1),
            "Test Category".to_string().into_boxed_str(),
            "Test Name".to_string().into_boxed_str(),
        );

        let stat = AnyStatistic::IntegerAmount(IntegerAmountStatistic::new(
            metadata.clone(),
            Arc::clone(&repo.logging_repositories.integer_amount_repo),
            Arc::clone(&repo.goal_repositories.integer_amount_repo),
        ));

        // Save the statistic
        repo.save(stat).unwrap();

        // Retrieve and verify
        let retrieved = repo.get_by_id(StatisticId(1)).unwrap();
        match retrieved {
            AnyStatistic::IntegerAmount(s) => {
                assert_eq!(s.id(), StatisticId(1));
                assert_eq!(s.get_name(), "Test Name");
                assert_eq!(s.get_category(), "Test Category");
            }
            _ => panic!("Wrong statistic type retrieved"),
        }
    }

    #[test]
    fn test_save_and_get_time_amount_statistic() {
        let pool = setup_test_db();
        let (logging_repos, goal_repos) = create_repositories(Arc::clone(&pool));
        let repo = StatisticRepository::new(pool, logging_repos, goal_repos);

        let metadata = StatisticMetadata::new(
            StatisticId(2),
            "Test Category".to_string().into_boxed_str(),
            "Time Test name".to_string().into_boxed_str(),
        );

        let stat = AnyStatistic::TimeAmount(TimeAmountStatistic::new(
            metadata.clone(),
            Arc::clone(&repo.logging_repositories.time_amount_repo),
            Arc::clone(&repo.goal_repositories.time_amount_repo),
        ));

        repo.save(stat).unwrap();

        let retrieved = repo.get_by_id(StatisticId(2)).unwrap();
        match retrieved {
            AnyStatistic::TimeAmount(s) => {
                assert_eq!(s.id(), StatisticId(2));
                assert_eq!(s.get_name(), "Time Test name");
            }
            _ => panic!("Wrong statistic type retrieved"),
        }
    }

    #[test]
    fn test_nonexistent_statistic() {
        let pool = setup_test_db();
        let (logging_repos, goal_repos) = create_repositories(Arc::clone(&pool));
        let repo = StatisticRepository::new(pool, logging_repos, goal_repos);

        let result = repo.get_by_id(StatisticId(999));
        assert!(result.is_err());
    }

    #[test]
    fn test_update_existing_statistic() {
        let pool = setup_test_db();
        let (logging_repos, goal_repos) = create_repositories(Arc::clone(&pool));
        let repo = StatisticRepository::new(pool, logging_repos, goal_repos);

        let metadata = StatisticMetadata::new(
            StatisticId(3),
            "Original Name".to_string().into_boxed_str(),
            "Test".to_string().into_boxed_str(),
        );

        let stat = AnyStatistic::Habit(HabitStatistic::new(
            metadata,
            Arc::clone(&repo.logging_repositories.habit_repo),
            Arc::clone(&repo.goal_repositories.habit_repo),
        ));

        // Initial save
        repo.save(stat).unwrap();

        // Update with new name
        let updated_metadata = StatisticMetadata::new(
            StatisticId(3),
            "Test".to_string().into_boxed_str(),
            "Updated Name".to_string().into_boxed_str(),
        );

        let updated_stat = AnyStatistic::Habit(HabitStatistic::new(
            updated_metadata,
            Arc::clone(&repo.logging_repositories.habit_repo),
            Arc::clone(&repo.goal_repositories.habit_repo),
        ));

        repo.save(updated_stat).unwrap();

        // Verify update
        let retrieved = repo.get_by_id(StatisticId(3)).unwrap();
        match retrieved {
            AnyStatistic::Habit(s) => {
                assert_eq!(s.get_name(), "Updated Name");
            }
            _ => panic!("Wrong statistic type retrieved"),
        }
    }
}
