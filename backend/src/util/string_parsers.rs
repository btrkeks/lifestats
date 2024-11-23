use anyhow::{anyhow, Result};
use chrono::{NaiveDate, NaiveTime};

pub fn parse_time(s: &str) -> std::result::Result<NaiveTime, chrono::ParseError> {
    NaiveTime::parse_from_str(s, "%H:%M:%S")
}

pub fn parse_date(s: &str) -> std::result::Result<NaiveDate, chrono::ParseError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
}

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
}
