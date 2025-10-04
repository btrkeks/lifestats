use std::sync::Arc;

use anyhow::{Context, Result};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::SqliteConnection;
use lifestats_domain::plot::repository::PlotRepository;
use lifestats_domain::plot::specification::PlotSpecification;
use lifestats_domain::shared::ids::{PlotId, StatisticId};

use crate::db::models::PlotRaw;
use crate::db::schema::plots;

type SqlitePool = Pool<ConnectionManager<SqliteConnection>>;

pub struct SqlitePlotRepository {
    pool: Arc<SqlitePool>,
}

impl SqlitePlotRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    fn connection(&self) -> Result<PooledConnection<ConnectionManager<SqliteConnection>>> {
        self.pool
            .get()
            .context("failed to get SQLite connection from pool")
    }
}

impl PlotRepository for SqlitePlotRepository {
    fn find_by_id(&self, id: PlotId) -> Result<Option<PlotSpecification>> {
        let mut conn = self.connection()?;

        let result = plots::table
            .find(id.0 as i32)
            .first::<PlotRaw>(&mut conn)
            .optional()
            .context("failed to query plot by id")?;

        match result {
            Some(raw) => Ok(Some(raw.into_plot_spec()?)),
            None => Ok(None),
        }
    }

    fn find_all(&self) -> Result<Vec<PlotSpecification>> {
        let mut conn = self.connection()?;

        let raws = plots::table
            .load::<PlotRaw>(&mut conn)
            .context("failed to load all plots")?;

        raws.into_iter().map(|raw| raw.into_plot_spec()).collect()
    }

    fn find_by_statistic_id(&self, statistic_id: StatisticId) -> Result<Vec<PlotSpecification>> {
        let mut conn = self.connection()?;

        let raws = plots::table
            .filter(plots::statistic_id.eq(statistic_id.0 as i32))
            .load::<PlotRaw>(&mut conn)
            .context("failed to load plots by statistic_id")?;

        raws.into_iter().map(|raw| raw.into_plot_spec()).collect()
    }

    fn save(&self, plot_spec: &PlotSpecification) -> Result<()> {
        let mut conn = self.connection()?;
        let raw = PlotRaw::from_plot_spec(plot_spec);

        diesel::insert_into(plots::table)
            .values(&raw)
            .on_conflict(plots::id)
            .do_update()
            .set(&raw)
            .execute(&mut conn)
            .context("failed to upsert plot specification")?;

        Ok(())
    }

    fn delete(&self, id: PlotId) -> Result<()> {
        let mut conn = self.connection()?;

        diesel::delete(plots::table.find(id.0 as i32))
            .execute(&mut conn)
            .context("failed to delete plot")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifestats_domain::plot::specification::PlotType;

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
