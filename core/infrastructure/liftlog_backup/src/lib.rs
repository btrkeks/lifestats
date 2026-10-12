//! Parses LiftLog remote-backup payloads.
//!
//! LiftLog's remote backup feature POSTs a gzipped, serialized SQLite database
//! (the app's full `db.db`). Completed workouts live in the `session` table as
//! one row per session: `id TEXT PRIMARY KEY, payload TEXT` where `payload` is
//! versioned JSON (see `app/src/models/storage/versions/` in the LiftLog repo).
//!
//! This crate implements the [`LiftLogBackupParser`] port: gunzip the payload,
//! open the snapshot as SQLite, and interpret each session payload into plain
//! workout facts (date, first/last set completion, total weight lifted).

mod session_payload;

use std::io::Read;

use diesel::prelude::*;
use diesel::sql_types::Text;
use flate2::read::GzDecoder;

use lifestats_application::error::{ApplicationError, ApplicationResult};
use lifestats_application::use_cases::integration::liftlog_backup_parser::{
    LiftLogBackupParser, LiftLogWorkout,
};

const SQLITE_MAGIC: &[u8] = b"SQLite format 3\0";
const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

#[derive(QueryableByName)]
struct SessionRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    payload: String,
}

/// Parses gzipped LiftLog SQLite snapshots via a temporary file and Diesel's
/// SQLite backend (the same backend the rest of the workspace links).
pub struct SqliteLiftLogBackupParser;

impl LiftLogBackupParser for SqliteLiftLogBackupParser {
    fn parse(&self, gzipped_backup: &[u8]) -> ApplicationResult<Vec<LiftLogWorkout>> {
        if gzipped_backup.len() < 2 || gzipped_backup[..2] != GZIP_MAGIC {
            return Err(ApplicationError::InvalidInput(
                "liftlog backup is not gzip data (missing 1f 8b magic bytes)".to_string(),
            ));
        }

        let mut database_bytes = Vec::new();
        GzDecoder::new(gzipped_backup)
            .read_to_end(&mut database_bytes)
            .map_err(|e| {
                ApplicationError::InvalidInput(format!("failed to gunzip liftlog backup: {e}"))
            })?;

        if !database_bytes.starts_with(SQLITE_MAGIC) {
            // Legacy LiftLog exports are protobuf; remote backup always sends SQLite.
            return Err(ApplicationError::InvalidInput(
                "liftlog backup is not a SQLite snapshot (legacy protobuf exports are not supported)"
                    .to_string(),
            ));
        }

        let rows = read_session_rows(&database_bytes)?;

        let mut workouts = Vec::new();
        let mut skipped = 0usize;
        for row in &rows {
            match session_payload::interpret(&row.payload) {
                Ok(Some(workout)) => workouts.push(workout),
                // Session with no completed set — nothing to log.
                Ok(None) => {}
                Err(error) => {
                    skipped += 1;
                    tracing::warn!(
                        session_id = %row.id,
                        %error,
                        "Skipping unparseable liftlog session payload"
                    );
                }
            }
        }

        if skipped > 0 {
            tracing::warn!(
                skipped,
                total = rows.len(),
                "Some liftlog sessions could not be parsed and were skipped"
            );
        }

        Ok(workouts)
    }
}

fn read_session_rows(database_bytes: &[u8]) -> ApplicationResult<Vec<SessionRow>> {
    let mut temp_file = tempfile::NamedTempFile::new().map_err(|e| {
        ApplicationError::Infrastructure(anyhow::anyhow!(
            "failed to create temp file for liftlog backup: {e}"
        ))
    })?;
    std::io::Write::write_all(&mut temp_file, database_bytes).map_err(|e| {
        ApplicationError::Infrastructure(anyhow::anyhow!(
            "failed to write liftlog backup to temp file: {e}"
        ))
    })?;

    let path = temp_file.path().to_string_lossy().into_owned();
    let mut connection = SqliteConnection::establish(&path).map_err(|e| {
        ApplicationError::InvalidInput(format!("liftlog backup is not a readable database: {e}"))
    })?;

    diesel::sql_query("SELECT id, payload FROM session")
        .load::<SessionRow>(&mut connection)
        .map_err(|e| {
            ApplicationError::InvalidInput(format!(
                "liftlog backup has no readable session table: {e}"
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(bytes).unwrap();
        encoder.finish().unwrap()
    }

    fn build_backup(payloads: &[(&str, &str)]) -> Vec<u8> {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let path = temp_file.path().to_string_lossy().into_owned();
        {
            let mut connection = SqliteConnection::establish(&path).unwrap();
            diesel::sql_query(
                "CREATE TABLE session (id TEXT PRIMARY KEY NOT NULL, payload TEXT NOT NULL)",
            )
            .execute(&mut connection)
            .unwrap();
            for (id, payload) in payloads {
                diesel::sql_query("INSERT INTO session (id, payload) VALUES (?, ?)")
                    .bind::<Text, _>(*id)
                    .bind::<Text, _>(*payload)
                    .execute(&mut connection)
                    .unwrap();
            }
        }
        gzip(&std::fs::read(&path).unwrap())
    }

    const SESSION_JSON: &str = r#"{
        "version": 4,
        "id": "s1",
        "blueprint": { "name": "Push", "notes": "" },
        "date": "2026-08-12",
        "recordedExercises": [
            {
                "type": "RecordedWeightedExercise",
                "blueprint": { "usesBodyweight": false },
                "potentialSets": [
                    {
                        "weight": { "unit": "kilograms", "value": "60" },
                        "set": { "repsCompleted": 10, "completionDateTime": "2026-08-12T18:00:00+02:00" }
                    },
                    {
                        "weight": { "unit": "kilograms", "value": "60" },
                        "set": { "repsCompleted": 8, "completionDateTime": "2026-08-12T18:12:00+02:00" }
                    }
                ]
            }
        ]
    }"#;

    #[test]
    fn parses_round_trip_backup() {
        let empty_session = r#"{"id": "s2", "date": "2026-08-13", "recordedExercises": []}"#;
        let broken_session = "not json";
        let backup = build_backup(&[
            ("s1", SESSION_JSON),
            ("s2", empty_session),
            ("s3", broken_session),
        ]);

        let workouts = SqliteLiftLogBackupParser.parse(&backup).unwrap();

        assert_eq!(workouts.len(), 1);
        let workout = &workouts[0];
        assert_eq!(workout.session_id, "s1");
        assert_eq!(workout.date.to_string(), "2026-08-12");
        assert_eq!(workout.started_at.to_string(), "2026-08-12 18:00:00");
        assert_eq!(workout.finished_at.to_string(), "2026-08-12 18:12:00");
        assert!((workout.total_weight_kg - 1080.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_payload_without_gzip_magic() {
        let error = SqliteLiftLogBackupParser
            .parse(b"plainly not gzip")
            .unwrap_err();
        assert!(matches!(error, ApplicationError::InvalidInput(_)));
    }

    #[test]
    fn rejects_gzipped_non_sqlite_payload() {
        let backup = gzip(b"this is a legacy protobuf export, honest");
        let error = SqliteLiftLogBackupParser.parse(&backup).unwrap_err();
        assert!(matches!(error, ApplicationError::InvalidInput(_)));
    }
}
