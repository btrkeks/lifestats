use chrono::NaiveTime;

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
