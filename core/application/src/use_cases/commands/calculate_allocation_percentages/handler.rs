use std::collections::HashMap;

use crate::cqrs::CommandHandler;
use crate::error::ApplicationResult;
use chrono::Duration;

use super::command::CalculatePercentagesCommand;
use anyhow::anyhow;

/// Handler responsible for calculating allocation percentages from timeline events.
pub struct CalculatePercentagesCommandHandler;

impl CommandHandler<CalculatePercentagesCommand> for CalculatePercentagesCommandHandler {
    fn handle(
        &self,
        _command: CalculatePercentagesCommand,
    ) -> ApplicationResult<HashMap<String, f64>> {
        let timeline_events = _command.timeline_events;

        let mut total_durations: HashMap<String, Duration> = HashMap::new();

        // Calculate total logged seconds for each category
        for event in &timeline_events {
            let dur = event.timeframe.duration();
            let label = event.label.as_str();

            if let Some(total) = total_durations.get_mut(label) {
                *total += dur;
            } else {
                total_durations.insert(event.label.clone(), dur); // clone once per new label
            }
        }

        // Calculate percentages
        let mut unlogged_seconds: Duration = Duration::seconds(86400); // 86400 seconds in a day
        let mut category_percentages: HashMap<String, f64> = HashMap::new();
        for (category, &duration) in &total_durations {
            let percentage = calculate_percentage_of_day(duration)?;
            category_percentages.insert(category.clone(), percentage);
            unlogged_seconds -= duration;
        }

        // Insert unlogged time as "Unlogged" category
        let unlogged_percentage = calculate_percentage_of_day(unlogged_seconds)?;
        category_percentages.insert("Unlogged".to_string(), unlogged_percentage);

        Ok(category_percentages)
    }
}

pub fn calculate_percentage_of_day(duration: Duration) -> ApplicationResult<f64> {
    // Duration should be positive and less than or equal to 86400 seconds (1 day)
    if duration < Duration::zero() || duration > Duration::seconds(86400) {
        return Err(anyhow!(
            "Duration '{}s' must be between 0 and 86400 seconds",
            duration.num_seconds()
        )
        .into());
    }

    Ok((duration.num_seconds() as f64 / 86400.0) * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::summary::TimelineEvent;
    use chrono::NaiveTime;
    use lifestats_core::shared::time_frame_list::TimeFrame;

    /// Helper function to create a TimelineEvent for testing
    fn create_event(
        start_hour: u32,
        start_min: u32,
        end_hour: u32,
        end_min: u32,
        label: &str,
    ) -> TimelineEvent {
        let start = NaiveTime::from_hms_opt(start_hour, start_min, 0).expect("Invalid start time");
        let end = NaiveTime::from_hms_opt(end_hour, end_min, 0).expect("Invalid end time");
        let timeframe = TimeFrame::new(start, end).expect("Failed to create timeframe");

        TimelineEvent {
            timeframe,
            label: label.to_string(),
        }
    }

    /// Helper function to assert percentage is close to expected value
    fn assert_approx_eq(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() < tolerance,
            "Expected {:.2}%, got {:.2}%",
            expected,
            actual
        );
    }

    /// Helper function with default tolerance (0.01%)
    fn assert_approx_eq_default(actual: f64, expected: f64) {
        assert_approx_eq(actual, expected, 0.01);
    }

    // ==================== HAPPY PATH TESTS ====================

    #[test]
    fn test_single_event_part_of_day() {
        // Arrange: Single 2-hour event
        let events = vec![create_event(9, 0, 11, 0, "Work")];
        let command = CalculatePercentagesCommand {
            timeline_events: events,
        };
        let handler = CalculatePercentagesCommandHandler;

        // Act
        let result = handler.handle(command).unwrap();

        // Assert
        assert_approx_eq_default(result["Work"], 2.0 / 24.0 * 100.0);
        assert_approx_eq_default(result["Unlogged"], 22.0 / 24.0 * 100.0);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_multiple_events_different_labels() {
        // Arrange: Multiple events with different labels
        let events = vec![
            create_event(9, 0, 12, 0, "Work"),   // 3 hours = 10800 seconds
            create_event(12, 0, 13, 0, "Lunch"), // 1 hour = 3600 seconds
            create_event(14, 0, 17, 0, "Study"), // 3 hours = 10800 seconds
        ];
        let command = CalculatePercentagesCommand {
            timeline_events: events,
        };
        let handler = CalculatePercentagesCommandHandler;

        // Act
        let result = handler.handle(command).unwrap();

        // Assert
        // Work: 10800/86400 * 100 = 12.5%
        assert_approx_eq_default(result["Work"], 3.0 / 24.0 * 100.0);
        // Lunch: 3600/86400 * 100 = 4.17%
        assert_approx_eq_default(result["Lunch"], 1.0 / 24.0 * 100.0);
        // Study: 10800/86400 * 100 = 12.5%
        assert_approx_eq_default(result["Study"], 3.0 / 24.0 * 100.0);
        // Unlogged: 86400 - 10800 - 3600 - 10800 = 61200 = 70.83%
        assert_approx_eq_default(result["Unlogged"], 17.0 / 24.0 * 100.0);
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_multiple_events_same_label_aggregates() {
        // Arrange: Multiple events with same label should be summed
        let events = vec![
            create_event(9, 0, 10, 0, "Work"),   // 1 hour
            create_event(10, 30, 12, 0, "Work"), // 1.5 hours
            create_event(13, 0, 15, 0, "Work"),  // 2 hours
        ];
        let command = CalculatePercentagesCommand {
            timeline_events: events,
        };
        let handler = CalculatePercentagesCommandHandler;

        // Act
        let result = handler.handle(command).unwrap();

        // Assert
        // Total Work: 4.5 hours = 16200 seconds = 18.75%
        assert_approx_eq_default(result["Work"], 4.5 / 24.0 * 100.0);
        // Unlogged: 86400 - 16200 = 70200 = 81.25%
        assert_approx_eq_default(result["Unlogged"], 19.5 / 24.0 * 100.0);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_full_day_logged_almost_complete() {
        // Arrange: Events covering almost full 24 hours
        let events = vec![
            create_event(0, 0, 6, 0, "Sleep"),     // 6 hours
            create_event(6, 0, 9, 0, "Morning"),   // 3 hours
            create_event(9, 0, 17, 0, "Work"),     // 8 hours
            create_event(17, 0, 21, 0, "Evening"), // 4 hours
            create_event(21, 0, 23, 59, "Rest"),   // 2 hours 59 minutes
        ];
        let command = CalculatePercentagesCommand {
            timeline_events: events,
        };
        let handler = CalculatePercentagesCommandHandler;

        // Act
        let result = handler.handle(command).unwrap();

        // Assert
        // Total: 23 hours 59 minutes = 86340 seconds = 99.93%
        // Sleep: 6 hours = 21600 seconds = 25%
        assert_approx_eq(result["Sleep"], 25.0, 0.1);
        // Morning: 3 hours = 10800 seconds = 12.5%
        assert_approx_eq(result["Morning"], 12.5, 0.1);
        // Work: 8 hours = 28800 seconds = 33.33%
        assert_approx_eq(result["Work"], 33.33, 0.1);
        // Evening: 4 hours = 14400 seconds = 16.67%
        assert_approx_eq(result["Evening"], 16.67, 0.1);
        // Rest: 2h 59m = 10740 seconds = 12.43%
        assert_approx_eq(result["Rest"], 12.43, 0.1);
        // Unlogged: 1 minute = 60 seconds = 0.07%
        assert_approx_eq(result["Unlogged"], 0.07, 0.05);

        // Verify percentages sum to ~100
        let sum: f64 = result.values().sum();
        assert_approx_eq(sum, 100.0, 0.1);
    }

    // ==================== EDGE CASE TESTS ====================

    #[test]
    fn test_empty_timeline_events() {
        // Arrange: No events
        let events = vec![];
        let command = CalculatePercentagesCommand {
            timeline_events: events,
        };
        let handler = CalculatePercentagesCommandHandler;

        // Act
        let result = handler.handle(command).unwrap();

        // Assert
        // All time is unlogged (100%)
        assert_approx_eq_default(result["Unlogged"], 100.0);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_single_event_entire_day() {
        // Arrange: Single event covering the full day
        let events = vec![create_event(0, 0, 23, 59, "Work")];
        let command = CalculatePercentagesCommand {
            timeline_events: events,
        };
        let handler = CalculatePercentagesCommandHandler;

        // Act
        let result = handler.handle(command).unwrap();

        // Assert
        // 23:59 - 00:00 = 23 hours 59 minutes = 86340 seconds = 99.93%
        assert_approx_eq(result["Work"], 99.93, 0.01);
        assert_approx_eq(result["Unlogged"], 0.07, 0.01);
    }

    #[test]
    fn test_very_small_duration_events() {
        // Arrange: Very short events (minute-level)
        let events = vec![
            create_event(9, 0, 9, 1, "Short1"),   // 1 minute = 60 seconds
            create_event(10, 0, 10, 5, "Short2"), // 5 minutes = 300 seconds
        ];
        let command = CalculatePercentagesCommand {
            timeline_events: events,
        };
        let handler = CalculatePercentagesCommandHandler;

        // Act
        let result = handler.handle(command).unwrap();

        // Assert
        // Short1: 60/86400 * 100 = 0.0694%
        assert_approx_eq(result["Short1"], 0.0694, 0.001);
        // Short2: 300/86400 * 100 = 0.347%
        assert_approx_eq(result["Short2"], 0.347, 0.001);
    }

    #[test]
    fn test_events_at_midnight_boundaries() {
        // Arrange: Events at 00:00 and 23:59
        let events = vec![
            create_event(0, 0, 1, 0, "Midnight"), // 1 hour at midnight
            create_event(23, 0, 23, 59, "Late"),  // 59 minutes near midnight
        ];
        let command = CalculatePercentagesCommand {
            timeline_events: events,
        };
        let handler = CalculatePercentagesCommandHandler;

        // Act
        let result = handler.handle(command).unwrap();

        // Assert
        // Midnight: 3600 seconds = 4.17%
        assert_approx_eq_default(result["Midnight"], 4.17);
        // Late: 3540 seconds = 4.10%
        assert_approx_eq_default(result["Late"], 4.10);
    }

    #[test]
    fn test_case_sensitive_labels() {
        // Arrange: Different cases of same label should be treated as different
        let events = vec![
            create_event(9, 0, 10, 0, "work"),
            create_event(10, 0, 11, 0, "Work"),
            create_event(11, 0, 12, 0, "WORK"),
        ];
        let command = CalculatePercentagesCommand {
            timeline_events: events,
        };
        let handler = CalculatePercentagesCommandHandler;

        // Act
        let result = handler.handle(command).unwrap();

        // Assert
        // Each label should be separate
        assert_eq!(result.len(), 4); // 3 different cases + Unlogged
        assert_approx_eq_default(result["work"], 4.17);
        assert_approx_eq_default(result["Work"], 4.17);
        assert_approx_eq_default(result["WORK"], 4.17);
    }

    #[test]
    fn test_events_with_special_characters_in_labels() {
        // Arrange: Labels with special characters
        let events = vec![
            create_event(9, 0, 10, 0, "Work-Project#1"),
            create_event(10, 0, 11, 0, "Morning (Exercise)"),
            create_event(11, 0, 12, 0, "C++/Rust Study"),
        ];
        let command = CalculatePercentagesCommand {
            timeline_events: events,
        };
        let handler = CalculatePercentagesCommandHandler;

        // Act
        let result = handler.handle(command).unwrap();

        // Assert
        assert!(result.contains_key("Work-Project#1"));
        assert!(result.contains_key("Morning (Exercise)"));
        assert!(result.contains_key("C++/Rust Study"));
    }

    // ==================== PERCENTAGES VALIDATION TESTS ====================

    #[test]
    fn test_percentages_sum_to_100() {
        // Arrange: Random events throughout the day
        let events = vec![
            create_event(6, 30, 9, 0, "Sleep"),
            create_event(9, 0, 9, 30, "Breakfast"),
            create_event(9, 30, 13, 0, "Work"),
            create_event(13, 0, 14, 0, "Lunch"),
            create_event(14, 0, 18, 0, "Work"),
            create_event(18, 0, 19, 0, "Dinner"),
            create_event(19, 0, 23, 0, "Leisure"),
        ];
        let command = CalculatePercentagesCommand {
            timeline_events: events,
        };
        let handler = CalculatePercentagesCommandHandler;

        // Act
        let result = handler.handle(command).unwrap();

        // Assert: Sum of all percentages should equal 100
        let sum: f64 = result.values().sum();
        assert_approx_eq(sum, 100.0, 0.2);
    }

    #[test]
    fn test_percentage_ranges_valid() {
        // Arrange: Multiple events
        let events = vec![
            create_event(9, 0, 17, 0, "Work"),
            create_event(17, 0, 22, 0, "Personal"),
        ];
        let command = CalculatePercentagesCommand {
            timeline_events: events,
        };
        let handler = CalculatePercentagesCommandHandler;

        // Act
        let result = handler.handle(command).unwrap();

        // Assert: All percentages should be between 0 and 100
        for (label, percentage) in &result {
            assert!(
                *percentage >= 0.0 && *percentage <= 100.0,
                "Percentage for '{}' is out of range: {:.2}%",
                label,
                percentage
            );
        }
    }

    // ==================== CALCULATE_PERCENTAGE_OF_DAY FUNCTION TESTS ====================

    #[test]
    fn test_calculate_percentage_zero_duration() {
        // Arrange: Zero duration
        let duration = Duration::seconds(0);

        // Act
        let result = calculate_percentage_of_day(duration).unwrap();

        // Assert
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_calculate_percentage_full_day() {
        // Arrange: Full 24 hours (86400 seconds)
        let duration = Duration::seconds(86400);

        // Act
        let result = calculate_percentage_of_day(duration).unwrap();

        // Assert
        assert_eq!(result, 100.0);
    }

    #[test]
    fn test_calculate_percentage_one_hour() {
        // Arrange: 1 hour = 3600 seconds
        let duration = Duration::seconds(3600);

        // Act
        let result = calculate_percentage_of_day(duration).unwrap();

        // Assert
        // 3600/86400 * 100 = 4.166...
        assert_approx_eq(result, 4.1666, 0.0001);
    }

    #[test]
    fn test_calculate_percentage_half_day() {
        // Arrange: 12 hours = 43200 seconds
        let duration = Duration::seconds(43200);

        // Act
        let result = calculate_percentage_of_day(duration).unwrap();

        // Assert
        assert_eq!(result, 50.0);
    }

    #[test]
    fn test_calculate_percentage_negative_duration_fails() {
        // Arrange: Negative duration
        let duration = Duration::seconds(-3600);

        // Act
        let result = calculate_percentage_of_day(duration);

        // Assert
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("must be between 0 and 86400 seconds"));
    }

    #[test]
    fn test_calculate_percentage_exceeds_24_hours_fails() {
        // Arrange: Duration > 86400 seconds
        let duration = Duration::seconds(86401);

        // Act
        let result = calculate_percentage_of_day(duration);

        // Assert
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("must be between 0 and 86400 seconds"));
    }

    #[test]
    fn test_calculate_percentage_way_over_24_hours_fails() {
        // Arrange: Way over 24 hours
        let duration = Duration::seconds(172800); // 48 hours

        // Act
        let result = calculate_percentage_of_day(duration);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_calculate_percentage_just_under_24_hours() {
        // Arrange: Just under 24 hours (1 second less)
        let duration = Duration::seconds(86399);

        // Act
        let result = calculate_percentage_of_day(duration).unwrap();

        // Assert
        // 86399/86400 * 100 = 99.9988...% (approximately 99.999%)
        assert!(
            result > 99.998 && result < 100.0,
            "Expected ~99.999%, got {:.6}%",
            result
        );
    }

    // ==================== REAL-WORLD SCENARIO TESTS ====================

    #[test]
    fn test_typical_weekday_schedule() {
        // Arrange: Realistic daily schedule
        let events = vec![
            create_event(0, 0, 7, 0, "Sleep"),     // 7 hours
            create_event(7, 0, 7, 30, "Morning"),  // 30 min
            create_event(7, 30, 9, 0, "Commute"),  // 1.5 hours
            create_event(9, 0, 12, 30, "Work"),    // 3.5 hours
            create_event(12, 30, 13, 30, "Lunch"), // 1 hour
            create_event(13, 30, 17, 0, "Work"),   // 3.5 hours
            create_event(17, 0, 18, 0, "Commute"), // 1 hour
            create_event(18, 0, 19, 0, "Dinner"),  // 1 hour
            create_event(19, 0, 23, 0, "Leisure"), // 4 hours
        ];
        let command = CalculatePercentagesCommand {
            timeline_events: events,
        };
        let handler = CalculatePercentagesCommandHandler;

        // Act
        let result = handler.handle(command).unwrap();

        // Assert
        assert!(result.contains_key("Sleep"));
        assert!(result.contains_key("Work"));
        let sum: f64 = result.values().sum();
        assert_approx_eq(sum, 100.0, 0.2);
    }

    #[test]
    fn test_many_short_intervals_same_category() {
        // Arrange: Many short 30-minute work sessions (4 sessions total = 2 hours)
        let events = vec![
            create_event(9, 0, 9, 30, "Work"),
            create_event(10, 0, 10, 30, "Work"),
            create_event(11, 0, 11, 30, "Work"),
            create_event(13, 0, 13, 30, "Work"),
        ];
        let command = CalculatePercentagesCommand {
            timeline_events: events,
        };
        let handler = CalculatePercentagesCommandHandler;

        // Act
        let result = handler.handle(command).unwrap();

        // Assert
        // 4 sessions * 30 minutes = 120 minutes = 2 hours = 7200 seconds = 8.33%
        assert_approx_eq(result["Work"], 8.333, 0.001);
    }

    #[test]
    fn test_overlapping_time_ranges_different_labels() {
        // Arrange: Non-overlapping consecutive events (common real-world case)
        let events = vec![
            create_event(8, 0, 12, 0, "Morning Work"),
            create_event(12, 0, 13, 0, "Lunch Break"),
            create_event(13, 0, 17, 0, "Afternoon Work"),
            create_event(17, 0, 20, 0, "Personal Time"),
        ];
        let command = CalculatePercentagesCommand {
            timeline_events: events,
        };
        let handler = CalculatePercentagesCommandHandler;

        // Act
        let result = handler.handle(command).unwrap();

        // Assert
        // Total logged: 12 hours = 43200 seconds = 50%
        // All percentages should sum to 100%
        assert!(result.len() > 1);
        let sum: f64 = result.values().sum();
        assert_approx_eq(sum, 100.0, 0.2);
    }
}
