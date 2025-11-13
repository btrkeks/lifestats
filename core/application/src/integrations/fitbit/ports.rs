use std::collections::HashSet;
use std::fmt;
use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, Utc};

use lifestats_core::shared::time_frame_list::{TimeFrame, TimeFrameList};

use crate::{
    error::{ApplicationError, ApplicationResult},
    integrations::tokens::{AccessToken, FitbitTokenData, RefreshToken},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum FitbitMetric {
    SleepDuration,
    WakeUpTime,
    FellAsleepTime,
    SleepEfficiency,
    TotalDurationAwake,
    Steps,
    CaloriesOut,
    ActiveMinutes,
    CardioMinutes,
    PeakMinutes,
}

impl FromStr for FitbitMetric {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sleep_duration" => Ok(FitbitMetric::SleepDuration),
            "wake_up_time" => Ok(FitbitMetric::WakeUpTime),
            "fell_asleep_time" => Ok(FitbitMetric::FellAsleepTime),
            "sleep_efficiency" => Ok(FitbitMetric::SleepEfficiency),
            "total_duration_awake" => Ok(FitbitMetric::TotalDurationAwake),
            "steps" => Ok(FitbitMetric::Steps),
            "calories_out" => Ok(FitbitMetric::CaloriesOut),
            "active_minutes" => Ok(FitbitMetric::ActiveMinutes),
            "cardio_minutes" => Ok(FitbitMetric::CardioMinutes),
            "peak_minutes" => Ok(FitbitMetric::PeakMinutes),
            _ => Err(format!("Unknown Fitbit metric: {}", s)),
        }
    }
}

impl fmt::Display for FitbitMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.canonical_tag_value())
    }
}

impl FitbitMetric {
    pub fn canonical_tag_value(&self) -> &'static str {
        match self {
            FitbitMetric::SleepDuration => "sleep_duration",
            FitbitMetric::WakeUpTime => "wake_up_time",
            FitbitMetric::FellAsleepTime => "fell_asleep_time",
            FitbitMetric::SleepEfficiency => "sleep_efficiency",
            FitbitMetric::TotalDurationAwake => "total_duration_awake",
            FitbitMetric::Steps => "steps",
            FitbitMetric::CaloriesOut => "calories_out",
            FitbitMetric::ActiveMinutes => "active_minutes",
            FitbitMetric::CardioMinutes => "cardio_minutes",
            FitbitMetric::PeakMinutes => "peak_minutes",
        }
    }

    pub fn required_scopes(&self) -> &'static [&'static str] {
        match self {
            FitbitMetric::SleepDuration
            | FitbitMetric::WakeUpTime
            | FitbitMetric::FellAsleepTime
            | FitbitMetric::SleepEfficiency
            | FitbitMetric::TotalDurationAwake => &["sleep"],
            FitbitMetric::Steps | FitbitMetric::CaloriesOut | FitbitMetric::ActiveMinutes => {
                &["activity"]
            }
            FitbitMetric::CardioMinutes | FitbitMetric::PeakMinutes => &["heartrate"],
        }
    }

    pub fn required_scope_set(&self) -> HashSet<&'static str> {
        self.required_scopes().iter().copied().collect()
    }
}

#[derive(Clone, Debug, Default)]
pub struct FitbitActivityData {
    pub steps: Option<u32>,
    pub calories_out: Option<u32>,
    pub active_minutes: Option<u32>,
    pub cardio_minutes: Option<u32>,
    pub peak_minutes: Option<u32>,
}

#[derive(Clone, Debug, Default)]
pub struct FitbitSleepData {
    pub total_duration_asleep: Option<ChronoDuration>,
    pub total_duration_awake: Option<ChronoDuration>,
    pub fell_asleep_at_utc: Option<DateTime<Utc>>,
    pub woke_up_at_utc: Option<DateTime<Utc>>,
    pub sleep_efficiency: Option<u8>,
}

impl FitbitSleepData {
    /// Extracts the main sleep period and converts it into a TimeFrameList.
    /// Returns Ok(None) if there is no main sleep period for the day.
    pub fn into_main_sleep_time_frames(&self) -> ApplicationResult<Option<TimeFrameList>> {
        if let (Some(start), Some(end)) = (self.fell_asleep_at_utc, self.woke_up_at_utc) {
            let time_frame = TimeFrame::new(start.time(), end.time())
                .map_err(|e| ApplicationError::InvalidInput(e.to_string()))?;

            let mut time_frame_list = TimeFrameList::default();
            time_frame_list
                .append_frame(time_frame)
                .map_err(|e| ApplicationError::InvalidInput(e.to_string()))?;

            Ok(Some(time_frame_list))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
pub trait FitbitOAuthApi: Send + Sync {
    async fn exchange_code_for_token(
        &self,
        code: &str,
        pkce_verifier: Option<String>,
    ) -> ApplicationResult<FitbitTokenData>;

    async fn refresh(&self, refresh_token: &RefreshToken) -> ApplicationResult<FitbitTokenData>;
}

#[async_trait]
pub trait FitbitDataApi: Send + Sync {
    async fn get_activity_data(
        &self,
        access_token: &AccessToken,
        date: NaiveDate,
    ) -> ApplicationResult<FitbitActivityData>;

    async fn get_sleep_data(
        &self,
        access_token: &AccessToken,
        date: NaiveDate,
    ) -> ApplicationResult<FitbitSleepData>;
}

#[async_trait]
pub trait TokenStorePort: Send + Sync {
    async fn load(&self, source: &str) -> ApplicationResult<Option<FitbitTokenData>>;
    async fn save(&self, source: &str, token: &FitbitTokenData) -> ApplicationResult<()>;
    async fn set_last_sync_at(&self, source: &str, t: DateTime<Utc>) -> ApplicationResult<()>;
    async fn last_sync_at(&self, source: &str) -> ApplicationResult<Option<DateTime<Utc>>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_into_main_sleep_time_frames_happy_path() {
        // Arrange: Create sleep data with valid start and end times on the same day
        // Note: TimeFrame only supports same-day ranges, so this represents a nap
        let sleep_data = FitbitSleepData {
            total_duration_asleep: None,
            total_duration_awake: None,
            fell_asleep_at_utc: Some(Utc.with_ymd_and_hms(2024, 1, 15, 14, 0, 0).unwrap()),
            woke_up_at_utc: Some(Utc.with_ymd_and_hms(2024, 1, 15, 16, 30, 0).unwrap()),
            sleep_efficiency: None,
        };

        // Act
        let result = sleep_data.into_main_sleep_time_frames();

        // Assert
        assert!(result.is_ok());
        let time_frame_list = result.unwrap();
        assert!(time_frame_list.is_some());

        let list = time_frame_list.unwrap();
        assert_eq!(list.len(), 1);

        let frames: Vec<_> = list.iter().collect();
        assert_eq!(
            frames[0].get_start(),
            chrono::NaiveTime::from_hms_opt(14, 0, 0).unwrap()
        );
        assert_eq!(
            frames[0].get_end(),
            chrono::NaiveTime::from_hms_opt(16, 30, 0).unwrap()
        );
    }

    #[test]
    fn test_into_main_sleep_time_frames_no_sleep_data() {
        // Arrange: Create sleep data with no start/end times
        let sleep_data = FitbitSleepData {
            total_duration_asleep: None,
            total_duration_awake: None,
            fell_asleep_at_utc: None,
            woke_up_at_utc: None,
            sleep_efficiency: None,
        };

        // Act
        let result = sleep_data.into_main_sleep_time_frames();

        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_into_main_sleep_time_frames_only_start_time() {
        // Arrange: Create sleep data with only start time (incomplete data)
        let sleep_data = FitbitSleepData {
            total_duration_asleep: None,
            total_duration_awake: None,
            fell_asleep_at_utc: Some(Utc.with_ymd_and_hms(2024, 1, 15, 22, 30, 0).unwrap()),
            woke_up_at_utc: None,
            sleep_efficiency: None,
        };

        // Act
        let result = sleep_data.into_main_sleep_time_frames();

        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_into_main_sleep_time_frames_only_end_time() {
        // Arrange: Create sleep data with only end time (incomplete data)
        let sleep_data = FitbitSleepData {
            total_duration_asleep: None,
            total_duration_awake: None,
            fell_asleep_at_utc: None,
            woke_up_at_utc: Some(Utc.with_ymd_and_hms(2024, 1, 16, 6, 15, 0).unwrap()),
            sleep_efficiency: None,
        };

        // Act
        let result = sleep_data.into_main_sleep_time_frames();

        // Assert
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_into_main_sleep_time_frames_invalid_order() {
        // Arrange: Create sleep data with end time before start time (same day)
        let sleep_data = FitbitSleepData {
            total_duration_asleep: None,
            total_duration_awake: None,
            fell_asleep_at_utc: Some(Utc.with_ymd_and_hms(2024, 1, 15, 6, 15, 0).unwrap()),
            woke_up_at_utc: Some(Utc.with_ymd_and_hms(2024, 1, 15, 3, 30, 0).unwrap()),
            sleep_efficiency: None,
        };

        // Act
        let result = sleep_data.into_main_sleep_time_frames();

        // Assert
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Start time cannot be later than end time"));
    }
}
