use chrono::{Duration, NaiveDate};
use plotters::prelude::*;

// Constants
pub const BACKGROUND_COLOR: RGBColor = RGBColor(37, 37, 37);
pub const GOAL_LINE_COLOR: RGBAColor = RGBAColor(255, 0, 0, 0.7);
pub const MISSED_EMPTY_GOAL_DAY_COLOR: RGBAColor = RGBAColor(245, 169, 142, 0.18);
pub const SHADING_COLOR: RGBAColor = RGBAColor(200, 200, 200, 0.1);

/// Creates a horizontal line element for plotters
pub fn get_horizontal_line(
    date: NaiveDate,
    height: i64,
    color: RGBAColor,
) -> PathElement<(SegmentValue<NaiveDate>, i64)> {
    PathElement::new(
        [
            (SegmentValue::Exact(date), height),
            (SegmentValue::Exact(date + Duration::days(1)), height),
        ],
        color.stroke_width(2),
    )
}

/// Creates a border rectangle for the plot
pub fn get_border(
    border_color: &RGBColor,
    x_axis_min: NaiveDate,
    x_axis_max: NaiveDate,
    y_axis_max: i64,
) -> Rectangle<(SegmentValue<NaiveDate>, i64)> {
    Rectangle::new(
        [
            (SegmentValue::Exact(x_axis_min), 0),
            (
                SegmentValue::Exact(x_axis_max + Duration::days(1)),
                y_axis_max,
            ),
        ],
        border_color.stroke_width(1),
    )
}

/// Creates a bar/rectangle element for a specific day
pub fn get_bar(
    date: NaiveDate,
    y0: i64,
    y1: i64,
    margin: u32,
    color: &RGBAColor,
) -> Rectangle<(SegmentValue<NaiveDate>, i64)> {
    let x0 = SegmentValue::Exact(date);
    let x1 = SegmentValue::Exact(date + Duration::days(1));
    let mut bar = Rectangle::new([(x0, y0), (x1, y1)], color.filled());
    bar.set_margin(0, 0, margin, margin);
    bar
}

/// Rounds a number up to the nearest multiple
fn round_up_to_multiple(num: i64, multiple: i64) -> i64 {
    ((num + multiple - 1) / multiple) * multiple
}

/// Gets tick marks for a given step size
fn get_ticks_for_step_size(y_min: i64, y_max: i64, step_size: i64) -> Vec<i64> {
    let first_tick = y_min + step_size - (y_min % step_size);
    debug_assert!(first_tick % step_size == 0);
    (first_tick..=y_max).step_by(step_size as usize).collect()
}

/// Calculates appropriate tick marks for time-based y-axis (in seconds)
pub fn calculate_seconds_ticks(y_min: i64, y_max: i64, plot_height: usize) -> Vec<i64> {
    let diff = y_max - y_min;

    let aim_number_of_ticks = (plot_height / 20) as i64;
    let corresp_step_size = diff / aim_number_of_ticks;

    let multiple: i64 = match corresp_step_size {
        ..=300 => 5 * 60, // <= 5 min
        301..=900 => 900, // <= 15 min
        _ => 1800,        // 30 min
    };
    let step_size = round_up_to_multiple(corresp_step_size, multiple);

    get_ticks_for_step_size(y_min, y_max, step_size)
}

/// Candidate tick steps for clock/duration axes, from one minute up to one day.
const NICE_SECONDS_STEPS: [i64; 13] = [
    60, 120, 300, 600, 900, 1800, 3600, 7200, 10800, 14400, 21600, 43200, 86400,
];

/// Calculates tick marks for a seconds-based y-axis that land on "round" times
/// (full minutes / quarter hours / hours ...) and keep at least `min_px_per_tick`
/// pixels between neighbouring ticks so labels stay readable.
pub fn nice_seconds_ticks(
    y_min: i64,
    y_max: i64,
    plot_height: u32,
    min_px_per_tick: u32,
) -> Vec<i64> {
    let range = (y_max - y_min).max(1);
    let max_ticks = (plot_height / min_px_per_tick.max(1)).max(1) as i64;
    let step = NICE_SECONDS_STEPS
        .iter()
        .copied()
        .find(|step| range / step <= max_ticks)
        .unwrap_or(*NICE_SECONDS_STEPS.last().unwrap());

    let first_tick = y_min.div_euclid(step) * step;
    let first_tick = if first_tick < y_min {
        first_tick + step
    } else {
        first_tick
    };
    (first_tick..=y_max).step_by(step as usize).collect()
}

/// Formats elapsed seconds as HH:MM. Durations may exceed 24 hours.
pub fn duration_seconds_to_hh_mm(total_sec: &i64) -> String {
    let total_minutes = total_sec.max(&0) / 60;
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    format!("{:02}:{:02}", hours, minutes)
}

/// Formats seconds from midnight as clock-style HH:MM.
pub fn time_of_day_seconds_to_hh_mm(total_sec: &i64) -> String {
    let seconds = total_sec.rem_euclid(24 * 60 * 60);
    let total_minutes = seconds / 60;
    let hours = total_minutes / 60;
    let minutes = total_minutes % 60;
    format!("{:02}:{:02}", hours, minutes)
}

pub fn number_formatter(value: &i64) -> String {
    value.to_string()
}

pub fn percent_formatter(value: &i64) -> String {
    format!("{}%", value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_seconds_to_hh_mm() {
        assert_eq!(duration_seconds_to_hh_mm(&0), "00:00");
        assert_eq!(duration_seconds_to_hh_mm(&3600), "01:00");
        assert_eq!(duration_seconds_to_hh_mm(&3660), "01:01");
        assert_eq!(duration_seconds_to_hh_mm(&99000), "27:30");
    }

    #[test]
    fn test_time_of_day_seconds_to_hh_mm() {
        assert_eq!(time_of_day_seconds_to_hh_mm(&0), "00:00");
        assert_eq!(time_of_day_seconds_to_hh_mm(&3600), "01:00");
        assert_eq!(time_of_day_seconds_to_hh_mm(&3660), "01:01");
        assert_eq!(time_of_day_seconds_to_hh_mm(&90000), "01:00");
    }

    #[test]
    fn nice_seconds_ticks_land_on_round_times() {
        // 23:20 .. 00:05 on a 500px plot with >= 40px spacing -> 5 min steps
        let ticks = nice_seconds_ticks(23 * 3600 + 20 * 60, 24 * 3600 + 5 * 60, 500, 40);
        assert_eq!(ticks.first(), Some(&(23 * 3600 + 20 * 60)));
        assert!(ticks.iter().all(|t| t % 300 == 0));
        assert!(ticks.len() <= 500 / 40);
        assert!(ticks.len() >= 3);
    }

    #[test]
    fn nice_seconds_ticks_use_hours_for_long_ranges() {
        let ticks = nice_seconds_ticks(0, 10 * 3600, 300, 40);
        assert!(ticks.len() <= 300 / 40);
        assert!(ticks.iter().all(|t| t % 3600 == 0));
    }

    #[test]
    fn nice_seconds_ticks_skip_below_range_start() {
        let ticks = nice_seconds_ticks(100, 1000, 600, 40);
        assert!(ticks.iter().all(|t| (100..=1000).contains(t)));
        assert_eq!(ticks.first(), Some(&120));
    }

    #[test]
    fn test_calculate_seconds_ticks() {
        let ticks = calculate_seconds_ticks(0, 3600, 300);
        assert!(!ticks.is_empty());
        // Ticks should be evenly spaced
        for window in ticks.windows(2) {
            let step = window[1] - window[0];
            assert!(step > 0);
            assert_eq!(step % 300, 0); // Should be multiple of 5 minutes
        }
    }
}
