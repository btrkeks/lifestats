use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

pub trait SleepResponse {
    fn get_total_duration_asleep(&self) -> Duration;
    fn get_sleep_efficiency(&self) -> Option<u8>;
    fn get_time_fell_asleep(&self) -> Option<NaiveTime>;
    fn get_wake_up_time(&self) -> Option<NaiveTime>;
    fn get_total_duration_awake_during_sleep(&self) -> Option<Duration>;
}

#[derive(Debug, Default, Deserialize)]
pub struct SleepResponseV1_2 {
    pub sleep: Vec<SleepData>,
    pub summary: SleepSummary,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SleepData {
    pub date_of_sleep: NaiveDate,
    pub duration: u64,
    pub efficiency: u8,
    pub end_time: NaiveDateTime,
    pub info_code: u8,
    pub is_main_sleep: bool,
    pub levels: SleepLevels,
    pub log_id: u64,
    pub log_type: String,
    pub minutes_after_wakeup: u32,
    pub minutes_asleep: u32,
    pub minutes_awake: u32,
    pub minutes_to_fall_asleep: u32,
    pub start_time: NaiveDateTime,
    pub time_in_bed: u32,
    #[serde(rename = "type")]
    pub sleep_type: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SleepLevels {
    pub data: Vec<LevelData>,
    pub short_data: Vec<LevelData>,
    pub summary: LevelsSummary,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SleepLevel {
    Deep,
    Light,
    Rem,
    #[default]
    Wake,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelData {
    pub date_time: NaiveDateTime,
    pub level: SleepLevel,
    pub seconds: u32,
}

#[derive(Debug, Default, Deserialize)]
pub struct LevelsSummary {
    pub deep: LevelSummary,
    pub light: LevelSummary,
    pub rem: LevelSummary,
    pub wake: LevelSummary,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelSummary {
    pub count: u32,
    pub minutes: u32,
    pub thirty_day_avg_minutes: f32,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SleepSummary {
    pub stages: StagesSummary,
    pub total_minutes_asleep: u32,
    pub total_sleep_records: u32,
    pub total_time_in_bed: u32,
}

#[derive(Default, Debug, Deserialize)]
pub struct StagesSummary {
    pub deep: u32,
    pub light: u32,
    pub rem: u32,
    pub wake: u32,
}

impl SleepResponse for SleepResponseV1_2 {
    fn get_total_duration_asleep(&self) -> chrono::Duration {
        Duration::minutes(self.summary.total_minutes_asleep as i64)
    }

    fn get_sleep_efficiency(&self) -> Option<u8> {
        self.sleep
            .iter()
            .find(|s| s.is_main_sleep)
            .map(|main_sleep| main_sleep.efficiency)
    }

    fn get_time_fell_asleep(&self) -> Option<NaiveTime> {
        self.sleep
            .iter()
            .find(|s| s.is_main_sleep)
            .and_then(|main_sleep| {
                main_sleep
                    .levels
                    .data
                    .iter()
                    .find(|stage| stage.level != SleepLevel::Wake && stage.seconds > 300)
                    .map(|level_data| level_data.date_time.time())
            })
    }

    fn get_wake_up_time(&self) -> Option<NaiveTime> {
        self.sleep
            .iter()
            .find(|s| s.is_main_sleep)
            .map(|main_sleep| main_sleep.end_time.time())
    }

    fn get_total_duration_awake_during_sleep(&self) -> Option<Duration> {
        let main_sleep = self.sleep.iter().find(|s| s.is_main_sleep);
        if let Some(sleep) = main_sleep {
            let total_seconds: u32 = sleep.levels.data.iter().map(|level| level.seconds).sum();

            Some(Duration::seconds(total_seconds as i64))
        } else {
            None
        }
    }
}

impl SleepResponseV1_2 {
    pub fn get_time_awake_between(&self, start: NaiveDateTime, end: NaiveDateTime) -> Duration {
        let main_sleep = self.sleep.iter().find(|s| s.is_main_sleep);
        let total_awake = end - start;

        if let Some(sleep) = main_sleep {
            let total_duration_not_awake = sleep
                .levels
                .data
                .iter()
                .filter(|level| level.level != SleepLevel::Wake && level.date_time < end)
                .fold(Duration::zero(), |acc, level| {
                    let level_start = level.date_time.max(start);
                    let level_end =
                        (level.date_time + Duration::seconds(level.seconds as i64)).min(end);
                    if level_start < level_end {
                        acc + (level_end - level_start)
                    } else {
                        acc
                    }
                });

            total_awake - total_duration_not_awake
        } else {
            total_awake
        }
    }
}
