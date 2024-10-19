use crate::objects::TimeFrame;
use anyhow::{anyhow, Context, Result};
use chrono::{Duration, NaiveTime};

pub fn parse_bool(s: &str) -> Result<bool> {
    match s.to_lowercase().as_str() {
        "true" | "t" | "yes" | "y" | "1" => Ok(true),
        "false" | "f" | "no" | "n" | "0" => Ok(false),
        _ => Err(anyhow!("'{}' is not a valid boolean value", s)),
    }
}

pub fn parse_unknown_time_format(input: &str) -> Option<NaiveTime> {
    let supported_formats = [
        "%H:%M:%S",    // 14:30:59
        "%H:%M",       // 14:30
        "%H:%M:%S%.f", // 14:30:59.123 (used by NaiveTime)
        "%I:%M %p",    // 2:30 PM
        "%I:%M:%S %p", // 2:30:59 PM
        "%r",          // 02:30:59 PM
    ];

    for format in &supported_formats {
        if let Ok(time) = NaiveTime::parse_from_str(input, format) {
            return Some(time);
        }
    }

    None
}

pub fn parse_time_frame(input: &str) -> Result<TimeFrame> {
    let parts: Vec<&str> = input.split('-').collect();
    if parts.len() != 2 {
        return Err(anyhow!(
            "Invalid time frame format. Expected 'HH:MM:SS-HH:MM:SS'"
        ));
    }

    let start_str = parts[0].trim();
    let end_str = parts[1].trim();

    let start = if start_str.is_empty() {
        return Err(anyhow!("Start time cannot be empty."));
    } else {
        parse_unknown_time_format(start_str).context(format!("Parsing start time '{start_str}'"))?
    };
    let end = if end_str.is_empty() {
        return Err(anyhow!("End time cannot be empty."));
    } else {
        parse_unknown_time_format(end_str).context(format!("Parsing end time '{end_str}'"))?
    };

    TimeFrame::new(start, end).context("Start time cannot be greater than end time.")
}

pub fn parse_duration(input: &str) -> Result<Duration> {
    if input.is_empty() {
        return Err(anyhow!("Empty input"));
    }

    if input.chars().all(|c| c.is_ascii_digit()) {
        let value: i64 = input.parse().map_err(|_| anyhow!("Invalid number"))?;
        return Ok(Duration::minutes(value));
    }

    let len = input.len();
    let (value_str, unit) = input.split_at(len - 1);
    let value: i64 = value_str.parse().map_err(|_| anyhow!("Invalid number"))?;

    match unit {
        "s" => Ok(Duration::seconds(value)),
        "m" => Ok(Duration::minutes(value)),
        "h" => Ok(Duration::hours(value)),
        "d" => Ok(Duration::days(value)),
        _ => Err(anyhow!("Invalid unit")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    #[test]
    fn test_parse_bool() {
        assert!(parse_bool("true").unwrap());
        assert!(parse_bool("TRUE").unwrap());
        assert!(parse_bool("t").unwrap());
        assert!(parse_bool("yes").unwrap());
        assert!(parse_bool("Y").unwrap());
        assert!(parse_bool("1").unwrap());

        assert!(!parse_bool("false").unwrap());
        assert!(!parse_bool("FALSE").unwrap());
        assert!(!parse_bool("f").unwrap());
        assert!(!parse_bool("no").unwrap());
        assert!(!parse_bool("N").unwrap());
        assert!(!parse_bool("0").unwrap());

        assert!(parse_bool("invalid").is_err());
    }

    #[test]
    fn test_parse_unknown_time_format() {
        assert_eq!(
            parse_unknown_time_format("14:30"),
            Some(NaiveTime::from_hms_opt(14, 30, 0).unwrap())
        );
        assert_eq!(
            parse_unknown_time_format("14:30:59"),
            Some(NaiveTime::from_hms_opt(14, 30, 59).unwrap())
        );
        assert_eq!(
            parse_unknown_time_format("2:30 PM"),
            Some(NaiveTime::from_hms_opt(14, 30, 0).unwrap())
        );
        assert_eq!(
            parse_unknown_time_format("2:30:59 PM"),
            Some(NaiveTime::from_hms_opt(14, 30, 59).unwrap())
        );
        assert_eq!(
            parse_unknown_time_format("14:30:59.123"),
            Some(NaiveTime::from_hms_milli_opt(14, 30, 59, 123).unwrap())
        );
        assert_eq!(
            parse_unknown_time_format("02:30:59 PM"),
            Some(NaiveTime::from_hms_opt(14, 30, 59).unwrap())
        );

        assert_eq!(parse_unknown_time_format("invalid"), None);
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::seconds(30));
        assert_eq!(parse_duration("45m").unwrap(), Duration::minutes(45));
        assert_eq!(parse_duration("2h").unwrap(), Duration::hours(2));
        assert_eq!(parse_duration("3d").unwrap(), Duration::days(3));

        assert!(parse_duration("30x").is_err());
        assert!(parse_duration("invalid").is_err());
    }
}
