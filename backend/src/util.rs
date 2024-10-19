use chrono::{Datelike, Duration, Local, NaiveDate, NaiveTime, Timelike};

pub fn get_date_today() -> NaiveDate {
    Local::now().date_naive()
}

pub fn get_time_now() -> NaiveTime {
    Local::now().time()
}

pub fn get_minutes_since_midnight() -> i32 {
    let now = get_time_now();
    (now.hour() * 60 + now.minute()) as i32
}

pub fn time_to_str(time: &NaiveTime) -> String {
    time.format("%H:%M:%S").to_string()
}

pub fn parse_time(s: &str) -> Result<NaiveTime, chrono::ParseError> {
    NaiveTime::parse_from_str(s, "%H:%M:%S")
}

pub fn parse_date(s: &str) -> Result<NaiveDate, chrono::ParseError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
}

pub fn get_num_of_days_for_closed_date_interval(start_date: NaiveDate, end_date: NaiveDate) -> i64 {
    (end_date - start_date).num_days() + 1
}

pub fn get_current_month_range() -> (NaiveDate, NaiveDate) {
    let now = Local::now().date_naive();
    let year = now.year();
    let month = now.month();

    let first_day = NaiveDate::from_ymd_opt(year, month, 1).expect("Invalid date");

    let last_day = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .expect("Invalid date")
    .pred_opt()
    .expect("Invalid date");

    (first_day, last_day)
}

// TODO: write cleanly
pub fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.num_seconds();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    let mut parts = Vec::new();

    if hours > 0 {
        parts.push(format!(
            "{} hour{}",
            hours,
            if hours != 1 { "s" } else { "" }
        ));
    }
    if minutes > 0 {
        parts.push(format!(
            "{} minute{}",
            minutes,
            if minutes != 1 { "s" } else { "" }
        ));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!(
            "{} second{}",
            seconds,
            if seconds != 1 { "s" } else { "" }
        ));
    }

    match parts.len() {
        0 => "0 seconds".to_string(),
        1 => parts[0].clone(),
        2 => format!("{} and {}", parts[0], parts[1]),
        _ => {
            let last = parts.pop().unwrap();
            format!("{}, and {}", parts.join(", "), last)
        }
    }
}
