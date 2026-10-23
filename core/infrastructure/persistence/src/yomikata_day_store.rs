use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Integer, Nullable, Text};
use lifestats_core::shared::ids::StatisticId;
use lifestats_core::statistic::any_statistic::AnyStatistic;
use lifestats_core::statistic::ports::error::RepositoryError;
use lifestats_core::yomikata::{
    replace_source_projection, YomikataBundleDefinitions, YomikataDayError, YomikataDayReplacement,
    YomikataDayStore, YomikataDayTotals,
};

use crate::db::{models::LoggingDataRaw, schema::logging_data};
use crate::repository::SqliteStatisticRepository;

#[derive(QueryableByName)]
struct Binding {
    #[diesel(sql_type = Nullable<Integer>)]
    statistic_id: Option<i32>,
}
#[derive(QueryableByName)]
struct Candidate {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Text)]
    statistic_type: String,
}
#[derive(QueryableByName)]
struct DayRow {
    #[diesel(sql_type = Text)]
    day: String,
    #[diesel(sql_type = BigInt)]
    cards_reviewed: i64,
    #[diesel(sql_type = BigInt)]
    study_time_seconds: i64,
}
impl DayRow {
    fn totals(&self) -> Result<YomikataDayTotals, YomikataDayError> {
        YomikataDayTotals::new(self.cards_reviewed, self.study_time_seconds)
            .map_err(|_| YomikataDayError::BundleConflict)
    }
    fn date(&self) -> Result<NaiveDate, YomikataDayError> {
        self.day
            .parse()
            .map_err(|_| YomikataDayError::BundleConflict)
    }
}

#[derive(Clone, Copy)]
enum Metric {
    Cards,
    Minutes,
}
impl Metric {
    fn tag(self) -> &'static str {
        match self {
            Self::Cards => "cards_reviewed",
            Self::Minutes => "study_minutes",
        }
    }
    fn projection(self, totals: YomikataDayTotals) -> i64 {
        match self {
            Self::Cards => totals.cards_reviewed(),
            Self::Minutes => totals.study_minutes(),
        }
    }
    fn legacy_totals(self, value: i64) -> Result<YomikataDayTotals, YomikataDayError> {
        let totals = match self {
            Self::Cards => YomikataDayTotals::new(value, 0),
            Self::Minutes => YomikataDayTotals::new(
                0,
                value
                    .checked_mul(60)
                    .ok_or(YomikataDayError::BundleConflict)?,
            ),
        };
        totals.map_err(|_| YomikataDayError::BundleConflict)
    }
}
fn storage(error: impl Into<anyhow::Error>) -> YomikataDayError {
    YomikataDayError::Storage(RepositoryError::OperationFailed(error.into()))
}

fn read_day(
    conn: &mut SqliteConnection,
    day: NaiveDate,
) -> Result<YomikataDayTotals, YomikataDayError> {
    let row = diesel::sql_query(
        "SELECT day,cards_reviewed,study_time_seconds FROM yomikata_day_totals WHERE day=?",
    )
    .bind::<Text, _>(day.to_string())
    .get_result::<DayRow>(conn)
    .optional()
    .map_err(storage)?;
    row.map(|row| row.totals())
        .transpose()
        .map(|totals| totals.unwrap_or_default())
}
fn write_day(
    conn: &mut SqliteConnection,
    day: NaiveDate,
    totals: YomikataDayTotals,
) -> Result<(), YomikataDayError> {
    diesel::sql_query("INSERT INTO yomikata_day_totals(day,cards_reviewed,study_time_seconds) VALUES(?,?,?) ON CONFLICT(day) DO UPDATE SET cards_reviewed=excluded.cards_reviewed,study_time_seconds=excluded.study_time_seconds")
        .bind::<Text,_>(day.to_string()).bind::<BigInt,_>(totals.cards_reviewed()).bind::<BigInt,_>(totals.study_time_seconds()).execute(conn).map_err(storage)?;
    Ok(())
}

enum ResolvedMetric {
    Bound(StatisticId),
    Recreated {
        id: StatisticId,
        other_days_changed: bool,
    },
}

fn resolve_metric(
    conn: &mut SqliteConnection,
    metric: Metric,
    definition: AnyStatistic,
    requested_day: NaiveDate,
) -> Result<ResolvedMetric, YomikataDayError> {
    let binding =
        diesel::sql_query("SELECT statistic_id FROM yomikata_metric_bindings WHERE metric=?")
            .bind::<Text, _>(metric.tag())
            .get_result::<Binding>(conn)
            .optional()
            .map_err(storage)?;
    let candidates=diesel::sql_query("SELECT id,statistic_type FROM statistics WHERE json_extract(tags,'$.source')='yomikata' AND json_extract(tags,'$.metric')=?")
        .bind::<Text,_>(metric.tag()).load::<Candidate>(conn).map_err(storage)?;
    if candidates.len() > 1
        || candidates
            .iter()
            .any(|c| c.statistic_type != "IntegerAmount")
    {
        return Err(YomikataDayError::BundleConflict);
    }
    if let Some(bound) = binding.as_ref().and_then(|b| b.statistic_id) {
        if candidates.first().map(|c| c.id) != Some(bound) {
            return Err(YomikataDayError::BundleConflict);
        }
    }
    let id = if let Some(candidate) = candidates.first() {
        StatisticId(candidate.id as u32)
    } else {
        SqliteStatisticRepository::insert_statistic_anyhow(conn, &definition)
            .map_err(storage)?
            .get_metadata()
            .id()
    };
    let mut changed = false;
    if binding.is_none() {
        let rows = logging_data::table
            .filter(logging_data::statistic_id.eq(id.0 as i32))
            .load::<LoggingDataRaw>(conn)
            .map_err(storage)?;
        for row in rows {
            let day = row
                .date
                .parse()
                .map_err(|_| YomikataDayError::BundleConflict)?;
            let value = serde_json::from_str::<i64>(&row.value)
                .map_err(|_| YomikataDayError::BundleConflict)?;
            let legacy = metric.legacy_totals(value)?;
            let current = read_day(conn, day)?;
            let totals = match metric {
                Metric::Cards => {
                    YomikataDayTotals::new(legacy.cards_reviewed(), current.study_time_seconds())?
                }
                Metric::Minutes => {
                    YomikataDayTotals::new(current.cards_reviewed(), legacy.study_time_seconds())?
                }
            };
            write_day(conn, day, totals)?;
        }
    } else if binding.as_ref().and_then(|b| b.statistic_id).is_none() {
        let rows = diesel::sql_query(
            "SELECT day,cards_reviewed,study_time_seconds FROM yomikata_day_totals",
        )
        .load::<DayRow>(conn)
        .map_err(storage)?;
        for row in rows {
            let day = row.date()?;
            if day == requested_day {
                continue;
            }
            let current =
                SqliteStatisticRepository::read_integer(conn, id, day).map_err(storage)?;
            let new = replace_source_projection(
                current.unwrap_or(0),
                0,
                metric.projection(row.totals()?),
            )
            .map_err(YomikataDayError::from)?;
            if current != Some(new) {
                SqliteStatisticRepository::write_integer(conn, id, day, new).map_err(storage)?;
                changed = true;
            }
        }
    }
    diesel::sql_query("INSERT INTO yomikata_metric_bindings(metric,statistic_id) VALUES(?,?) ON CONFLICT(metric) DO UPDATE SET statistic_id=excluded.statistic_id")
        .bind::<Text,_>(metric.tag()).bind::<Integer,_>(id.0 as i32).execute(conn).map_err(storage)?;
    Ok(match binding {
        Some(Binding { statistic_id: None }) => ResolvedMetric::Recreated {
            id,
            other_days_changed: changed,
        },
        _ => ResolvedMetric::Bound(id),
    })
}

impl YomikataDayStore for SqliteStatisticRepository {
    fn replace_day(
        &self,
        day: NaiveDate,
        totals: YomikataDayTotals,
        definitions: YomikataBundleDefinitions,
    ) -> Result<YomikataDayReplacement, YomikataDayError> {
        let mut conn = self.connection()?;
        conn.immediate_transaction::<_, anyhow::Error, _>(|conn| {
            let cards = resolve_metric(
                conn,
                Metric::Cards,
                AnyStatistic::IntegerAmount(definitions.cards_reviewed),
                day,
            )?;
            let minutes = resolve_metric(
                conn,
                Metric::Minutes,
                AnyStatistic::IntegerAmount(definitions.study_minutes),
                day,
            )?;
            let old = read_day(conn, day)?;
            let mut changed_statistics = Vec::new();
            for (metric, resolved) in [(Metric::Cards, cards), (Metric::Minutes, minutes)] {
                let (id, old_projection, rebuilt) = match resolved {
                    ResolvedMetric::Bound(id) => (id, metric.projection(old), false),
                    ResolvedMetric::Recreated {
                        id,
                        other_days_changed,
                    } => (id, 0, other_days_changed),
                };
                let current =
                    SqliteStatisticRepository::read_integer(conn, id, day).map_err(storage)?;
                let new = replace_source_projection(
                    current.unwrap_or(0),
                    old_projection,
                    metric.projection(totals),
                )
                .map_err(YomikataDayError::from)?;
                if current != Some(new) {
                    SqliteStatisticRepository::write_integer(conn, id, day, new)
                        .map_err(storage)?;
                }
                if rebuilt || current != Some(new) {
                    changed_statistics.push(id);
                }
            }
            write_day(conn, day, totals)?;
            Ok(YomikataDayReplacement { changed_statistics })
        })
        .map_err(|error| match error.downcast::<YomikataDayError>() {
            Ok(error) => error,
            Err(error) => storage(error),
        })
    }
}
