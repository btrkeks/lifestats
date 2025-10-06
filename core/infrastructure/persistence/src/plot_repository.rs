use std::sync::Arc;

use anyhow::Context;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::SqliteConnection;
use lifestats_core::shared::ids::{PlotId, StatisticId};
use lifestats_core::statistic::ports::error::RepositoryError;
use plot_domain::plot::repository::PlotRepository;
use plot_domain::plot::specification::PlotSpecification;

use crate::db::models::PlotRaw;
use crate::db::schema::plots;

type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

// Helper function to convert diesel errors to RepositoryError
fn diesel_to_repository_error(err: diesel::result::Error) -> RepositoryError {
    match err {
        diesel::result::Error::NotFound => RepositoryError::NotFound,
        _ => RepositoryError::OperationFailed(err.into()),
    }
}

pub struct SqlitePlotRepository {
    pool: Arc<SqlitePool>,
}

impl SqlitePlotRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    fn connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<SqliteConnection>>, RepositoryError> {
        self.pool
            .get()
            .context("failed to get SQLite connection from pool")
            .map_err(RepositoryError::OperationFailed)
    }
}

impl PlotRepository for SqlitePlotRepository {
    fn find_by_id(&self, id: PlotId) -> Result<Option<PlotSpecification>, RepositoryError> {
        let mut conn = self.connection()?;

        let result = plots::table
            .find(id.0 as i32)
            .first::<PlotRaw>(&mut conn)
            .optional()
            .map_err(diesel_to_repository_error)?;

        match result {
            Some(raw) => Ok(Some(
                raw.into_plot_spec()
                    .map_err(RepositoryError::OperationFailed)?,
            )),
            None => Ok(None),
        }
    }

    fn find_all(&self) -> Result<Vec<PlotSpecification>, RepositoryError> {
        let mut conn = self.connection()?;

        let raws = plots::table
            .load::<PlotRaw>(&mut conn)
            .map_err(diesel_to_repository_error)?;

        raws.into_iter()
            .map(|raw| {
                raw.into_plot_spec()
                    .map_err(RepositoryError::OperationFailed)
            })
            .collect()
    }

    fn find_by_statistic_id(
        &self,
        statistic_id: StatisticId,
    ) -> Result<Vec<PlotSpecification>, RepositoryError> {
        let mut conn = self.connection()?;

        let raws = plots::table
            .filter(plots::statistic_id.eq(statistic_id.0 as i32))
            .load::<PlotRaw>(&mut conn)
            .map_err(diesel_to_repository_error)?;

        raws.into_iter()
            .map(|raw| {
                raw.into_plot_spec()
                    .map_err(RepositoryError::OperationFailed)
            })
            .collect()
    }

    fn save(&self, plot_spec: &PlotSpecification) -> Result<(), RepositoryError> {
        let mut conn = self.connection()?;
        let raw = PlotRaw::from_plot_spec(plot_spec);

        diesel::insert_into(plots::table)
            .values(&raw)
            .on_conflict(plots::id)
            .do_update()
            .set(&raw)
            .execute(&mut conn)
            .map_err(diesel_to_repository_error)?;

        Ok(())
    }

    fn delete(&self, id: PlotId) -> Result<(), RepositoryError> {
        let mut conn = self.connection()?;

        diesel::delete(plots::table.find(id.0 as i32))
            .execute(&mut conn)
            .map_err(diesel_to_repository_error)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plot_domain::plot::specification::PlotType;

    #[test]
    fn test_plot_raw_conversion() {
        let spec = PlotSpecification::new(
            PlotId(1),
            "Test Plot".to_string(),
            StatisticId(42),
            PlotType::LineChart,
        );

        let raw = PlotRaw::from_plot_spec(&spec);
        assert_eq!(raw.id, 1);
        assert_eq!(raw.statistic_id, 42);
        assert_eq!(raw.title, "Test Plot");
        assert_eq!(raw.plot_type, "line_chart");

        let spec2 = raw.into_plot_spec().unwrap();
        assert_eq!(spec2.id, spec.id);
        assert_eq!(spec2.statistic_id, spec.statistic_id);
        assert_eq!(spec2.title, spec.title);
        assert_eq!(spec2.plot_type, spec.plot_type);
    }
}
