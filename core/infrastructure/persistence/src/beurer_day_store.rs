use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::sql_types::{Integer, Text};
use lifestats_core::beurer::{
    BeurerBundleDefinitions, BeurerDayError, BeurerDayReading, BeurerDayReplacement,
    BeurerDayStore, METRICS,
};
use lifestats_core::shared::{ids::StatisticId, ports::error::RepositoryError};
use lifestats_core::statistic::any_statistic::AnyStatistic;

use crate::repository::SqliteStatisticRepository;

#[derive(QueryableByName)]
struct Candidate {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Text)]
    statistic_type: String,
}

fn storage(error: impl Into<anyhow::Error>) -> BeurerDayError {
    BeurerDayError::Storage(RepositoryError::OperationFailed(error.into()))
}

fn resolve_metric(
    conn: &mut SqliteConnection,
    metric: &str,
    definition: AnyStatistic,
) -> Result<StatisticId, BeurerDayError> {
    let candidates = diesel::sql_query("SELECT id,statistic_type FROM statistics WHERE json_extract(tags,'$.source')='beurer' AND json_extract(tags,'$.metric')=?")
        .bind::<Text, _>(metric).load::<Candidate>(conn).map_err(storage)?;
    if candidates.len() > 1
        || candidates
            .iter()
            .any(|candidate| candidate.statistic_type != "IntegerAmount")
    {
        return Err(BeurerDayError::BundleConflict);
    }
    if let Some(candidate) = candidates.first() {
        Ok(StatisticId(candidate.id as u32))
    } else {
        Ok(
            SqliteStatisticRepository::insert_statistic_anyhow(conn, &definition)
                .map_err(storage)?
                .get_metadata()
                .id(),
        )
    }
}

impl BeurerDayStore for SqliteStatisticRepository {
    fn replace_day(
        &self,
        day: NaiveDate,
        reading: BeurerDayReading,
        definitions: BeurerBundleDefinitions,
    ) -> Result<BeurerDayReplacement, BeurerDayError> {
        let mut conn = self.connection()?;
        conn.immediate_transaction::<_, anyhow::Error, _>(|conn| {
            let mut changed_statistics = Vec::new();
            for ((metric, definition), value) in
                METRICS.into_iter().zip(definitions.0).zip(reading.values())
            {
                let id = resolve_metric(conn, metric, AnyStatistic::IntegerAmount(definition))?;
                if let Some(value) = value {
                    let current =
                        SqliteStatisticRepository::read_integer(conn, id, day).map_err(storage)?;
                    if current != Some(value) {
                        SqliteStatisticRepository::write_integer(conn, id, day, value)
                            .map_err(storage)?;
                        changed_statistics.push(id);
                    }
                }
            }
            Ok(BeurerDayReplacement { changed_statistics })
        })
        .map_err(|error| match error.downcast::<BeurerDayError>() {
            Ok(error) => error,
            Err(error) => storage(error),
        })
    }
}
