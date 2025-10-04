use chrono::{Duration, NaiveDate};
use plotters::prelude::*;

// Constants
pub const BACKGROUND_COLOR: RGBColor = RGBColor(37, 37, 37);
pub const GOAL_LINE_COLOR: RGBAColor = RGBAColor(255, 0, 0, 0.7);
pub const SHADING_COLOR: RGBAColor = RGBAColor(200, 200, 200, 0.1);
pub const Y_MARGIN_PERCENT: i64 = 10;

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

/// Adds padding to y-axis bounds based on a percentage
pub fn add_y_padding(y0: i64, y1: i64, percent: i64) -> (i64, i64) {
    let diff = (y0 - y1).abs();

    debug_assert!((1..100).contains(&percent));
    let denom = 100 - 2 * percent;

    if denom <= 0 {
        return (y0, y1);
    }

    let padding = (diff * percent) / denom;

    (y0 - padding, y1 + padding)
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

/// Formats seconds as HH:MM time string
pub fn seconds_to_time_formatter(total_sec: &i64) -> String {
    const FORMATTER: [u8; 5] = [b'0'; 5];
    let mut buf = FORMATTER;

    let mins = (total_sec / 60) % 60;
    let hours = total_sec / 3600;

    let h1 = (hours / 10) as u8 + b'0';
    let h2 = ((hours % 10) % 24) as u8 + b'0';

    let m1 = (mins / 10) as u8 + b'0';
    let m2 = (mins % 10) as u8 + b'0';

    buf[0] = h1;
    buf[1] = h2;
    buf[2] = b':';
    buf[3] = m1;
    buf[4] = m2;

    unsafe { String::from_utf8_unchecked(buf.to_vec()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_y_padding_basic() {
        let (min, max) = add_y_padding(0, 100, 10);
        // For 10% padding: diff=100, denom=80, padding=(100*10)/80=12
        assert_eq!(min, -12);
        assert_eq!(max, 112);
    }

    #[test]
    fn test_seconds_to_time_formatter() {
        assert_eq!(seconds_to_time_formatter(&0), "00:00");
        assert_eq!(seconds_to_time_formatter(&3600), "01:00");
        assert_eq!(seconds_to_time_formatter(&3660), "01:01");
        assert_eq!(seconds_to_time_formatter(&36000), "10:00");
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
