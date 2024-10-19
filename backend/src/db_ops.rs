use crate::db_events::{publish_db_event, Event};
use crate::objects::{StatisticRow, TimeFrame, TimeFrameList};
use crate::paths;
use anyhow::{anyhow, Context, Result};
use chrono::{Duration, NaiveDate, NaiveTime};
use rusqlite::types::ValueRef;
use rusqlite::{params, Connection, OpenFlags};
use std::path::PathBuf;
use thiserror::Error;

type DbResult<T> = Result<T, DbError>;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    SqliteError(#[from] rusqlite::Error),

    #[error("Failed to parse JSON: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Failed to open database at path: {path}")]
    DatabaseOpenError {
        path: PathBuf,
        source: rusqlite::Error,
    },

    #[error("Failed to initialize database: {0}")]
    DatabaseInitError(String),
}

// TODO: Is this really necessary?
fn initialize_database(conn: &Connection) -> DbResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS logging_data (
            statistic_id INTEGER,
            date TEXT,
            value TEXT,
            goal TEXT,
            PRIMARY KEY (statistic_id, date))",
        [],
    )?;

    Ok(())
}

pub fn open_db() -> Result<Connection> {
    let db_path = paths::get_db_path().context("Failed to get database path")?;

    let conn = Connection::open(&db_path)
        .with_context(|| format!("Failed to open database at path: {:?}", db_path))?;

    initialize_database(&conn).context("Failed to initialize database")?;

    Ok(conn)
}

pub fn open_db_readonly() -> Result<Connection> {
    let db_path = paths::get_db_path().context("Failed to get database path")?;

    Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("Failed to open database at path: {:?}", db_path))
}

pub fn set_goal<T: DbStorable>(
    conn: &Connection,
    id: u32,
    date: NaiveDate,
    goal: Option<T>,
) -> Result<()> {
    let goal_str = goal.as_ref().map(|g| g.to_db_string());

    conn.execute(
        "INSERT OR REPLACE INTO logging_data (statistic_id, date, value, goal)
         VALUES (
             ?1,
             ?2,
             COALESCE((SELECT value FROM logging_data WHERE statistic_id = ?1 AND date = ?2), NULL),
             ?3
         )",
        params![id, date.to_string(), goal_str],
    )
    .context("Failed to update goal in database")?;

    Ok(())
}

// Includes both start_date and end_date
pub fn get_rows_for_range<T: DbStorable + Clone>(
    conn: &Connection, // Accept both Connection and Transaction
    id: u32,
    start_date: NaiveDate,
    end_date: NaiveDate,
    default_value: Option<T>,
    default_goal: Option<T>,
) -> Result<Vec<StatisticRow<T>>> {
    let mut stmt = conn
        .prepare(
            "SELECT date, value, goal
                 FROM logging_data
                 WHERE statistic_id = ?1 AND date BETWEEN ?2 AND ?3
                 ORDER BY date",
        )
        .context("Preparing SQL statement")?;

    let db_rows = stmt
        .query_map(
            params![id, start_date.to_string(), end_date.to_string()],
            |row| {
                let date_str: String = row.get(0)?;
                let value_str: Option<String> = row.get(1)?;
                let goal_str: Option<String> = row.get(2)?;

                let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                let value = value_str
                    .map(|s| T::from_db_string(&s))
                    .transpose()
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;

                let goal = goal_str
                    .map(|s| T::from_db_string(&s))
                    .transpose()
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;

                Ok(StatisticRow::new(date, value, goal))
            },
        )
        .context("Executing query")?;

    let days_count = end_date.signed_duration_since(start_date).num_days() as usize + 1;
    let mut db_rows = db_rows.peekable();

    let result: Vec<StatisticRow<T>> = (0..=days_count)
        .map(|day| {
            let current_date = start_date + Duration::days(day as i64);

            match db_rows.peek() {
                Some(Ok(row)) if row.date == current_date => {
                    // Safe to unwrap here because we just peeked and saw it's Some(Ok(_))
                    Ok(db_rows.next().unwrap().unwrap())
                }
                Some(Ok(row)) if row.date > current_date => Ok(StatisticRow::new(
                    current_date,
                    default_value.clone(),
                    default_goal.clone(),
                )),
                Some(Ok(_)) => Err(anyhow::anyhow!(
                    "BUG: Database returned a date earlier than the requested range"
                )),
                Some(Err(e)) => Err(anyhow::anyhow!("Database error: {}", e)),
                None => Ok(StatisticRow::new(
                    current_date,
                    default_value.clone(),
                    default_goal.clone(),
                )),
            }
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(result)
}

pub fn get_row_for_date<T: DbStorable>(
    conn: &Connection, // Accept both Connection and Transaction
    id: u32,
    date: NaiveDate,
) -> Result<Option<StatisticRow<T>>> {
    let mut stmt = conn
        .prepare(
            "
        SELECT date, value, goal
        FROM logging_data
        WHERE statistic_id = ?1 AND date = ?2
    ",
        )
        .context("Preparing SQL statement")?;

    let result = stmt.query_row(params![id, date.to_string()], |row| {
        let date_str: String = row.get(0)?;
        let value_str: String = row.get(1)?;
        let goal_str: Option<String> = match row.get_ref(2)? {
            ValueRef::Null => None,
            ValueRef::Text(bytes) => Some(String::from_utf8_lossy(bytes).to_string()),
            _ => {
                return Err(rusqlite::Error::InvalidColumnType(
                    3,
                    "goal".into(),
                    rusqlite::types::Type::Text,
                ))
            }
        };

        let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

        let value = T::from_db_string(&value_str)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;

        let goal = goal_str
            .map(|s| T::from_db_string(&s))
            .transpose()
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;

        Ok(StatisticRow::new_with_value(date, value, goal))
    });

    match result {
        Ok(row) => Ok(Some(row)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context(format!("Querying data for date: {}", date)),
    }
}

pub fn insert_row<T: DbStorable>(conn: &Connection, id: u32, row: &StatisticRow<T>) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO logging_data (statistic_id, date, value, goal)
         VALUES (?, ?, ?, ?)",
        params![
            id,
            row.date.to_string(),
            row.value.as_ref().map(|value| value.to_db_string()),
            row.goal.as_ref().map(|g| g.to_db_string()),
        ],
    )?;

    publish_db_event(Event::RowUpdated(id));
    Ok(())
}

pub trait DbStorable: Sized {
    fn from_db_string(value: &str) -> Result<Self>;
    fn to_db_string(&self) -> String;
}

impl DbStorable for i64 {
    fn from_db_string(value: &str) -> Result<Self> {
        value
            .parse()
            .map_err(|e| anyhow::anyhow!("Failed to parse i64: {}", e))
    }

    fn to_db_string(&self) -> String {
        self.to_string()
    }
}

impl DbStorable for TimeFrameList {
    fn from_db_string(string: &str) -> Result<Self> {
        serde_json::from_str(string).context("Failed to deserialize TimeFrameList")
    }

    fn to_db_string(&self) -> String {
        serde_json::to_string(self)
            .context("Failed to serialize TimeFrameList")
            .unwrap() // 危ないかも
    }
}

impl DbStorable for TimeFrame {
    fn from_db_string(string: &str) -> Result<Self> {
        serde_json::from_str(string).context("Failed to deserialize TimeFrameList")
    }

    fn to_db_string(&self) -> String {
        serde_json::to_string(self)
            .context("Failed to serialize TimeFrame")
            .unwrap() // 危ないかも
    }
}

impl DbStorable for Duration {
    fn from_db_string(value: &str) -> Result<Self> {
        let seconds: i64 = value.parse()?;
        Ok(Duration::seconds(seconds))
    }

    fn to_db_string(&self) -> String {
        self.num_seconds().to_string()
    }
}

impl DbStorable for NaiveTime {
    fn from_db_string(value: &str) -> Result<Self> {
        NaiveTime::parse_from_str(value, "%H:%M:%S").with_context(|| {
            format!(
                "Failed to parse time string '{}'. Expected format: HH:MM:SS",
                value
            )
        })
    }

    fn to_db_string(&self) -> String {
        self.format("%H:%M:%S").to_string()
    }
}

impl DbStorable for bool {
    fn from_db_string(value: &str) -> Result<Self> {
        match value {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => Err(anyhow!("Failed to parse bool: invalid value '{}'", value)),
        }
    }

    fn to_db_string(&self) -> String {
        if *self { "1" } else { "0" }.to_string()
    }
}
