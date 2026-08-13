//! Interprets a LiftLog `session.payload` JSON blob into workout facts.
//!
//! The payload schema is LiftLog's versioned `SessionJSON` (currently v4).
//! Parsing is deliberately lenient: only the fields needed here are read,
//! unknown exercise types are ignored, and optional fields default. This keeps
//! the parser working across payload versions (v1 payloads have no `version`
//! field; newer versions may add fields).

use chrono::{DateTime, NaiveDate, NaiveDateTime};
use serde::Deserialize;

use lifestats_application::use_cases::integration::LiftLogWorkout;

const POUNDS_TO_KG: f64 = 0.453_592_37;
const WEIGHTED_EXERCISE_TYPE: &str = "RecordedWeightedExercise";

#[derive(Deserialize)]
struct SessionPayload {
    id: String,
    /// ISO local date, e.g. "2026-08-13" (no time, no zone).
    date: String,
    #[serde(default)]
    bodyweight: Option<WeightJson>,
    #[serde(rename = "recordedExercises", default)]
    recorded_exercises: Vec<RecordedExercise>,
}

/// One struct for both exercise variants instead of a tagged enum, so unknown
/// future `type` values degrade to "no sets" instead of failing the session.
#[derive(Deserialize)]
struct RecordedExercise {
    #[serde(rename = "type")]
    exercise_type: Option<String>,
    #[serde(default)]
    blueprint: Option<ExerciseBlueprint>,
    /// Weighted exercises: configured sets, completed ones carry `set`.
    #[serde(rename = "potentialSets", default)]
    potential_sets: Vec<PotentialSet>,
    /// Cardio exercises.
    #[serde(default)]
    sets: Vec<CardioSet>,
}

#[derive(Deserialize)]
struct ExerciseBlueprint {
    #[serde(rename = "usesBodyweight", default)]
    uses_bodyweight: bool,
}

#[derive(Deserialize)]
struct PotentialSet {
    #[serde(default)]
    set: Option<RecordedSet>,
    #[serde(default)]
    weight: Option<WeightJson>,
}

#[derive(Deserialize)]
struct RecordedSet {
    #[serde(rename = "repsCompleted", default)]
    reps_completed: i64,
    /// ISO offset datetime, e.g. "2026-08-13T18:04:22.123+02:00".
    #[serde(rename = "completionDateTime")]
    completion_date_time: Option<String>,
}

#[derive(Deserialize)]
struct CardioSet {
    #[serde(rename = "completionDateTime")]
    completion_date_time: Option<String>,
}

/// LiftLog `WeightJSON`: `{ unit: 'kilograms'|'pounds'|'nil', value: "12.5" }`
/// where `value` is always a decimal string.
#[derive(Deserialize)]
struct WeightJson {
    #[serde(default)]
    unit: Option<String>,
    #[serde(default)]
    value: Option<String>,
}

impl WeightJson {
    fn kilograms(&self) -> f64 {
        let value: f64 = self
            .value
            .as_deref()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0);
        match self.unit.as_deref() {
            Some("pounds") => value * POUNDS_TO_KG,
            // 'kilograms', 'nil' (unit unknown, value is 0 in practice), or absent
            _ => value,
        }
    }
}

/// Returns `Ok(None)` for sessions without a single completed set.
pub fn interpret(payload_json: &str) -> anyhow::Result<Option<LiftLogWorkout>> {
    let payload: SessionPayload = serde_json::from_str(payload_json)?;

    let date = NaiveDate::parse_from_str(&payload.date, "%Y-%m-%d")?;

    let mut completions: Vec<NaiveDateTime> = Vec::new();
    let mut total_weight_kg = 0.0f64;
    let bodyweight_kg = payload
        .bodyweight
        .as_ref()
        .map(WeightJson::kilograms)
        .unwrap_or(0.0);

    for exercise in &payload.recorded_exercises {
        let is_weighted = exercise.exercise_type.as_deref() == Some(WEIGHTED_EXERCISE_TYPE);
        let uses_bodyweight = exercise
            .blueprint
            .as_ref()
            .is_some_and(|b| b.uses_bodyweight);

        for potential_set in &exercise.potential_sets {
            let Some(recorded) = &potential_set.set else {
                continue;
            };
            if let Some(time) = parse_completion(recorded.completion_date_time.as_deref()) {
                completions.push(time);
            }
            if is_weighted {
                // Mirror LiftLog's totalWeightLifted: effective weight is the
                // stored (added) weight, plus bodyweight for bodyweight-based
                // exercises, times completed reps. Cardio contributes nothing.
                let set_weight_kg = potential_set
                    .weight
                    .as_ref()
                    .map(WeightJson::kilograms)
                    .unwrap_or(0.0);
                let effective_kg = if uses_bodyweight {
                    set_weight_kg + bodyweight_kg
                } else {
                    set_weight_kg
                };
                total_weight_kg += effective_kg * recorded.reps_completed as f64;
            }
        }

        for cardio_set in &exercise.sets {
            if let Some(time) = parse_completion(cardio_set.completion_date_time.as_deref()) {
                completions.push(time);
            }
        }
    }

    let (Some(started_at), Some(finished_at)) = (
        completions.iter().min().copied(),
        completions.iter().max().copied(),
    ) else {
        return Ok(None);
    };

    Ok(Some(LiftLogWorkout {
        session_id: payload.id,
        date,
        started_at,
        finished_at,
        total_weight_kg,
    }))
}

/// Completion timestamps carry the device's UTC offset at recording time; we
/// keep the local wall-clock component, matching how the user experienced the
/// workout and how LiftLog attributes the session date.
fn parse_completion(raw: Option<&str>) -> Option<NaiveDateTime> {
    let raw = raw?;
    DateTime::parse_from_rfc3339(raw)
        .map(|dt| dt.naive_local())
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weighted_session() -> &'static str {
        r#"{
            "version": 4,
            "id": "11111111-2222-3333-4444-555555555555",
            "blueprint": { "name": "Push Day", "notes": "" },
            "date": "2026-08-12",
            "bodyweight": { "unit": "kilograms", "value": "80" },
            "recordedExercises": [
                {
                    "type": "RecordedWeightedExercise",
                    "blueprint": { "name": "Bench Press", "usesBodyweight": false },
                    "potentialSets": [
                        {
                            "weight": { "unit": "kilograms", "value": "60" },
                            "set": { "repsCompleted": 10, "completionDateTime": "2026-08-12T18:00:00.000+02:00" }
                        },
                        {
                            "weight": { "unit": "kilograms", "value": "60" },
                            "set": { "repsCompleted": 8, "completionDateTime": "2026-08-12T18:05:30.000+02:00" }
                        },
                        { "weight": { "unit": "kilograms", "value": "60" } }
                    ]
                },
                {
                    "type": "RecordedWeightedExercise",
                    "blueprint": { "name": "Pull Ups", "usesBodyweight": true },
                    "potentialSets": [
                        {
                            "weight": { "unit": "kilograms", "value": "10" },
                            "set": { "repsCompleted": 5, "completionDateTime": "2026-08-12T18:20:00.000+02:00" }
                        }
                    ]
                }
            ]
        }"#
    }

    #[test]
    fn interprets_weighted_session() {
        let workout = interpret(weighted_session()).unwrap().unwrap();

        assert_eq!(workout.session_id, "11111111-2222-3333-4444-555555555555");
        assert_eq!(workout.date, NaiveDate::from_ymd_opt(2026, 8, 12).unwrap());
        assert_eq!(
            workout.started_at,
            NaiveDate::from_ymd_opt(2026, 8, 12)
                .unwrap()
                .and_hms_opt(18, 0, 0)
                .unwrap()
        );
        assert_eq!(
            workout.finished_at,
            NaiveDate::from_ymd_opt(2026, 8, 12)
                .unwrap()
                .and_hms_opt(18, 20, 0)
                .unwrap()
        );
        // 60*10 + 60*8 + (10 + 80 bodyweight)*5 = 600 + 480 + 450
        assert!((workout.total_weight_kg - 1530.0).abs() < 1e-9);
    }

    #[test]
    fn converts_pounds_and_ignores_unknown_exercise_types() {
        let json = r#"{
            "id": "s2",
            "date": "2026-08-13",
            "recordedExercises": [
                {
                    "type": "RecordedWeightedExercise",
                    "blueprint": { "usesBodyweight": false },
                    "potentialSets": [
                        {
                            "weight": { "unit": "pounds", "value": "100" },
                            "set": { "repsCompleted": 2, "completionDateTime": "2026-08-13T07:00:00+02:00" }
                        }
                    ]
                },
                { "type": "SomeFutureExerciseType" }
            ]
        }"#;

        let workout = interpret(json).unwrap().unwrap();
        assert!((workout.total_weight_kg - 2.0 * 100.0 * POUNDS_TO_KG).abs() < 1e-9);
    }

    #[test]
    fn cardio_only_session_has_times_but_no_weight() {
        let json = r#"{
            "id": "s3",
            "date": "2026-08-13",
            "recordedExercises": [
                {
                    "type": "RecordedCardioExercise",
                    "sets": [
                        { "completionDateTime": "2026-08-13T09:10:00+02:00" },
                        { "completionDateTime": "2026-08-13T09:40:00+02:00" },
                        {}
                    ]
                }
            ]
        }"#;

        let workout = interpret(json).unwrap().unwrap();
        assert_eq!(workout.total_weight_kg, 0.0);
        assert_eq!(workout.started_at.time().to_string(), "09:10:00");
        assert_eq!(workout.finished_at.time().to_string(), "09:40:00");
    }

    #[test]
    fn session_without_completed_sets_is_skipped() {
        let json = r#"{
            "id": "s4",
            "date": "2026-08-13",
            "recordedExercises": [
                {
                    "type": "RecordedWeightedExercise",
                    "potentialSets": [ { "weight": { "unit": "kilograms", "value": "60" } } ]
                }
            ]
        }"#;

        assert!(interpret(json).unwrap().is_none());
    }

    #[test]
    fn legacy_payload_without_version_field_parses() {
        // v1 payloads have no `version` key and may lack newer fields.
        let json = r#"{
            "id": "s5",
            "date": "2024-01-05",
            "recordedExercises": [
                {
                    "type": "RecordedWeightedExercise",
                    "blueprint": {},
                    "potentialSets": [
                        {
                            "weight": { "unit": "nil", "value": "0" },
                            "set": { "repsCompleted": 12, "completionDateTime": "2024-01-05T17:30:00+01:00" }
                        }
                    ]
                }
            ]
        }"#;

        let workout = interpret(json).unwrap().unwrap();
        assert_eq!(workout.date, NaiveDate::from_ymd_opt(2024, 1, 5).unwrap());
        assert_eq!(workout.total_weight_kg, 0.0);
    }
}
