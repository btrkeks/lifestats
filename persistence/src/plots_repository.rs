use crate::sqlite_database::models::PlotRaw;
use crate::sqlite_database::schema::plots;
use anyhow::{anyhow, Context, Result};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use domain_models::plot::any_plot::AnyPlot;
use domain_models::plot::objects::plot_metadata::PlotMetadata;
use domain_models::plot::plot_backends::plotters::PlottersBackend;
use domain_models::plot::plot_types::bar::BarPlot;
use domain_models::plot::plot_types::circular_progress_bar::CircularProgressBarPlot;
use domain_models::plot::plot_types::scatter::ScatterPlot;
use domain_models::plot::plot_types::time_frames::TimeFramesPlot;
use domain_models::repositories::plots_repository::PlotRepositoryTrait;
use domain_models::repositories::statistic_repository::StatisticRepositoryTrait;
use std::sync::Arc;
use transfer_objects::statistic_id::{PlotId, StatisticId};

// TODO: Reconsider injecting strategy

pub struct PlotRepository {
    pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
    statistic_repository: Arc<dyn StatisticRepositoryTrait>,
}

impl PlotRepository {
    pub fn new(
        pool: Arc<Pool<ConnectionManager<SqliteConnection>>>,
        statistic_repository: Arc<dyn StatisticRepositoryTrait>,
    ) -> Self {
        Self {
            pool,
            statistic_repository,
        }
    }
}

impl PlotRepositoryTrait for PlotRepository {
    fn get_by_id(&self, id: PlotId) -> Result<Option<AnyPlot>> {
        let mut conn = self
            .pool
            .get()
            .context("Failed to get database connection")?;

        let plot_raw = plots::table
            .filter(plots::id.eq(id.0 as i32))
            .first::<PlotRaw>(&mut conn)
            .optional()
            .context("Failed to query plot by ID")?;

        match plot_raw {
            Some(p) => Ok(Some(raw_to_any_plot(
                p,
                Arc::clone(&self.statistic_repository),
            )?)),
            None => Ok(None),
        }
    }

    fn get_all(&self) -> Result<Box<[AnyPlot]>> {
        let mut conn = self
            .pool
            .get()
            .context("Failed to get database connection")?;

        let raw_plots = plots::table
            .load::<PlotRaw>(&mut conn)
            .context("Failed to query all plots")?;

        raw_plots
            .into_iter()
            .map(|p| raw_to_any_plot(p, Arc::clone(&self.statistic_repository)))
            .collect()
    }

    fn get_for_statistic_id(&self, stat_id: StatisticId) -> Result<Box<[AnyPlot]>> {
        let mut conn = self
            .pool
            .get()
            .context("Failed to get database connection")?;

        let raw_plots = plots::table
            .filter(plots::statistic_id.eq(stat_id.0 as i32))
            .load::<PlotRaw>(&mut conn)
            .context("Failed to query plots by statistic ID")?;

        raw_plots
            .into_iter()
            .map(|p| raw_to_any_plot(p, Arc::clone(&self.statistic_repository)))
            .collect()
    }

    fn save(&self, plot: AnyPlot) -> Result<()> {
        let mut conn = self
            .pool
            .get()
            .context("Failed to get database connection")?;

        let plot_raw = PlotRaw::try_from(&plot)?;

        diesel::insert_into(plots::table)
            .values(&plot_raw)
            .on_conflict(plots::id)
            .do_update()
            .set(&plot_raw)
            .execute(&mut conn)
            .context("Failed to insert or update plot")?;

        Ok(())
    }
}

fn raw_to_any_plot(
    plot_raw: PlotRaw,
    statistic_repository: Arc<dyn StatisticRepositoryTrait>,
) -> Result<AnyPlot> {
    let metadata = PlotMetadata {
        id: PlotId(plot_raw.id as u32),
        title: plot_raw.title,
        statistic_id: StatisticId(plot_raw.statistic_id as u32),
    };

    let backend = PlottersBackend;
    let statistic_id = StatisticId(plot_raw.statistic_id as u32);

    match plot_raw.plot_type.as_str() {
        "Bar" => Ok(AnyPlot::Bar(BarPlot {
            metadata,
            statistic_id,
            size: (plot_raw.width as usize, plot_raw.height as usize),
            statistic_repository: statistic_repository.clone(),
            plotting_backend: Arc::new(backend),
        })),
        "CircularProgressBar" => Ok(AnyPlot::CircularProgressBar(CircularProgressBarPlot {
            metadata,
            statistic_id,
            size: (plot_raw.width as usize, plot_raw.height as usize),
            statistic_repository: statistic_repository.clone(),
            backend: Arc::new(backend),
        })),
        "Scatter" => Ok(AnyPlot::Scatter(ScatterPlot {
            metadata,
            statistic_id,
            size: (plot_raw.width as usize, plot_raw.height as usize),
            statistic_repository: statistic_repository.clone(),
            plotting_backend: Arc::new(backend),
        })),
        "TimeFrames" => Ok(AnyPlot::TimeFrames(TimeFramesPlot {
            metadata,
            statistic_id,
            size: (plot_raw.width as usize, plot_raw.height as usize),
            statistic_repository: statistic_repository.clone(),
            plotting_backend: Arc::new(backend),
        })),
        _ => Err(anyhow!("Unknown plot type: {}", plot_raw.plot_type)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::MIGRATIONS;
    use diesel::r2d2::ConnectionManager;
    use diesel_migrations::MigrationHarness;
    use domain_models::plot::any_plot::AnyPlot;
    use domain_models::plot::objects::plot_metadata::PlotMetadata;
    use domain_models::plot::plot_backends::plotters::PlottersBackend;
    use domain_models::plot::plot_types::bar::BarPlot;
    use domain_models::repositories::statistic_repository::StatisticRepositoryTrait;
    use domain_models::statistic::statistic_types::any_statistic::AnyStatistic;
    use domain_models::statistic::statistic_types::time_spans::TimeSpansStatistic;
    use std::sync::Arc;
    use transfer_objects::statistic_id::{PlotId, StatisticId};

    // Create a mock StatisticRepository for testing
    struct MockStatisticRepository;
    impl StatisticRepositoryTrait for MockStatisticRepository {
        fn get_by_id(&self, _id: StatisticId) -> Result<AnyStatistic> {
            unimplemented!()
        }

        fn save(&self, _statistic: AnyStatistic) -> Result<()> {
            unimplemented!()
        }

        fn get_all(&self) -> Result<Box<[AnyStatistic]>> {
            unimplemented!()
        }

        fn get_all_time_spans_statistics(&self) -> Result<Box<[TimeSpansStatistic]>> {
            unimplemented!()
        }
    }

    // Helper function to set up an in-memory database for testing
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
    fn test_save_and_get_plot() {
        let pool = setup_test_db();
        let statistic_repository = Arc::new(MockStatisticRepository);
        let backend = PlottersBackend;

        let plot_repository = PlotRepository::new(pool, statistic_repository);

        let metadata = PlotMetadata {
            id: PlotId(1),
            title: "Test Plot".to_string(),
            statistic_id: StatisticId(1),
        };
        let plot = AnyPlot::Bar(BarPlot {
            metadata,
            statistic_id: StatisticId(1),
            size: (400, 250),
            statistic_repository: Arc::new(MockStatisticRepository),
            plotting_backend: Arc::new(backend),
        });

        plot_repository.save(plot).unwrap();

        let retrieved_plot = plot_repository.get_by_id(PlotId(1)).unwrap().unwrap();

        assert_eq!(retrieved_plot.get_id(), PlotId(1));
        assert_eq!(retrieved_plot.get_title(), "Test Plot");
        assert_eq!(retrieved_plot.get_statistic_id(), StatisticId(1));
    }

    #[test]
    fn test_get_all_plots() {
        let pool = setup_test_db();
        let statistic_repository = Arc::new(MockStatisticRepository);
        let plot_repository = PlotRepository::new(pool, statistic_repository);

        let metadata1 = PlotMetadata {
            id: PlotId(1),
            title: "Test Plot 1".to_string(),
            statistic_id: StatisticId(1),
        };
        let metadata2 = PlotMetadata {
            id: PlotId(2),
            title: "Test Plot 2".to_string(),
            statistic_id: StatisticId(2),
        };
        let backend = PlottersBackend;
        let plot1 = AnyPlot::Bar(BarPlot {
            metadata: metadata1,
            statistic_id: StatisticId(1),
            size: (400, 250),
            statistic_repository: Arc::new(MockStatisticRepository),
            plotting_backend: Arc::new(backend.clone()),
        });
        let plot2 = AnyPlot::Scatter(ScatterPlot {
            metadata: metadata2,
            statistic_id: StatisticId(2),
            size: (400, 250),
            statistic_repository: Arc::new(MockStatisticRepository),
            plotting_backend: Arc::new(backend),
        });

        plot_repository.save(plot1).unwrap();
        plot_repository.save(plot2).unwrap();

        let all_plots = plot_repository.get_all().unwrap();

        assert_eq!(all_plots.len(), 2);
    }

    #[test]
    fn test_get_plots_for_statistic_id() {
        let pool = setup_test_db();
        let statistic_repository = Arc::new(MockStatisticRepository);
        let plot_repository = PlotRepository::new(pool, statistic_repository);

        let metadata1 = PlotMetadata {
            id: PlotId(1),
            title: "Test Plot 1".to_string(),
            statistic_id: StatisticId(1),
        };
        let metadata2 = PlotMetadata {
            id: PlotId(2),
            title: "Test Plot 2".to_string(),
            statistic_id: StatisticId(2),
        };
        let metadata3 = PlotMetadata {
            id: PlotId(3),
            title: "Test Plot 3".to_string(),
            statistic_id: StatisticId(1), // Same statistic_id as plot1
        };
        let backend = PlottersBackend;
        let plot1 = AnyPlot::Bar(BarPlot {
            metadata: metadata1,
            statistic_id: StatisticId(1),
            size: (400, 250),
            statistic_repository: Arc::new(MockStatisticRepository),
            plotting_backend: Arc::new(backend.clone()),
        });
        let plot2 = AnyPlot::Scatter(ScatterPlot {
            metadata: metadata2,
            statistic_id: StatisticId(2),
            size: (400, 250),
            statistic_repository: Arc::new(MockStatisticRepository),
            plotting_backend: Arc::new(backend.clone()),
        });
        let plot3 = AnyPlot::TimeFrames(TimeFramesPlot {
            metadata: metadata3,
            statistic_id: StatisticId(1),
            size: (400, 250),
            statistic_repository: Arc::new(MockStatisticRepository),
            plotting_backend: Arc::new(backend),
        });

        plot_repository.save(plot1).unwrap();
        plot_repository.save(plot2).unwrap();
        plot_repository.save(plot3).unwrap();

        let plots_for_stat_1 = plot_repository
            .get_for_statistic_id(StatisticId(1))
            .unwrap();
        assert_eq!(plots_for_stat_1.len(), 2);

        let plots_for_stat_2 = plot_repository
            .get_for_statistic_id(StatisticId(2))
            .unwrap();
        assert_eq!(plots_for_stat_2.len(), 1);
    }

    #[test]
    fn test_save_plot_update() {
        let pool = setup_test_db();
        let statistic_repository = Arc::new(MockStatisticRepository);
        let plot_repository = PlotRepository::new(pool, statistic_repository);

        let metadata = PlotMetadata {
            id: PlotId(1),
            title: "Test Plot".to_string(),
            statistic_id: StatisticId(1),
        };
        let backend = PlottersBackend;
        let plot = AnyPlot::Bar(BarPlot {
            metadata,
            statistic_id: StatisticId(1),
            size: (400, 250),
            statistic_repository: Arc::new(MockStatisticRepository),
            plotting_backend: Arc::new(backend.clone()),
        });

        plot_repository.save(plot).unwrap();

        // Update the plot (e.g., change the title)
        let updated_metadata = PlotMetadata {
            id: PlotId(1),
            title: "Updated Test Plot".to_string(),
            statistic_id: StatisticId(1),
        };
        let updated_plot = AnyPlot::Scatter(ScatterPlot {
            metadata: updated_metadata,
            statistic_id: StatisticId(1),
            size: (400, 250),
            statistic_repository: Arc::new(MockStatisticRepository),
            plotting_backend: Arc::new(backend),
        });

        plot_repository.save(updated_plot).unwrap();

        let retrieved_plot = plot_repository.get_by_id(PlotId(1)).unwrap().unwrap();

        // Check that the plot has been updated
        assert_eq!(retrieved_plot.get_title(), "Updated Test Plot");
        assert!(matches!(retrieved_plot, AnyPlot::Scatter(_)));
    }

    #[test]
    fn test_get_nonexistent_plot() {
        let pool = setup_test_db();
        let statistic_repository = Arc::new(MockStatisticRepository);
        let plot_repository = PlotRepository::new(pool, statistic_repository);

        let result = plot_repository.get_by_id(PlotId(999)).unwrap();

        assert!(result.is_none());
    }
}
