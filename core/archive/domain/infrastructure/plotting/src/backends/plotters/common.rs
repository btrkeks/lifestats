use crate::plot::objects;
use crate::plot::objects::plot_data::PlotDataPoint;
use crate::plot::plot_backends::plotters::constants::Y_MARGIN_PERCENT;
use chrono::{Duration, NaiveDate};
use itertools::Itertools;
use log::error;
use plotters::prelude::*;

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

pub fn calculate_y_axis_bounds(data: &[PlotDataPoint<i64>]) -> (i64, i64) {
    let minmax = calculate_y_min_max(data);

    if let Some((y_min, y_max)) = minmax {
        add_y_padding(y_min, y_max, Y_MARGIN_PERCENT)
    } else {
        // Return some default bounds
        (0, 100)
    }
}

pub(crate) fn calculate_y_min_max(data: &[PlotDataPoint<i64>]) -> Option<(i64, i64)> {
    data.iter()
        .flat_map(|point| [point.value, point.goal])
        .flatten()
        .minmax()
        .into_option()
}

pub fn add_y_padding(y0: i64, y1: i64, percent: i64) -> (i64, i64) {
    // TODO: Switch y0 and y1 if y1 < y0
    let diff = (y0 - y1).abs();

    debug_assert!(1 <= percent && percent < 100);
    let denom = 100 - 2 * percent;

    // Avoid division by zero if `percent == 100` (or > 100).
    if denom <= 0 {
        error!("Invalid percentage for y-axis padding: {}", percent);
        return (y0, y1);
    }

    let padding = (diff * percent) / denom; // Note: Integer division!

    (y0 - padding, y1 + padding)
}

pub fn color_to_plotters(color: &objects::color::Color) -> RGBColor {
    let (r, g, b) = color.get_rgb();
    RGBColor(r, g, b)
}

fn get_number_of_digits(number: u32) -> u32 {
    if number == 0 {
        return 1;
    }

    (number as f64).log10().ceil() as u32
}

pub fn get_streak_text(image_width: u32, streak_value: u32) -> Text<'static, (i32, i32), String> {
    let num_of_digits = get_number_of_digits(streak_value);
    // TODO: Plotters doesn't seem to provide a way to measure the font width
    let x_offset = 55 + (num_of_digits * 6);

    Text::new(
        format!("連続: {}", streak_value),
        ((image_width - x_offset) as i32, 10),
        ("IPAMincho", 18).into_font().color(&WHITE),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plot::objects::color;

    #[test]
    fn test_add_y_padding_basic() {
        let (min, max) = add_y_padding(0, 100, 10);
        // For 10% padding: range is 100, padding should be (100 * 10/90) ≈ 11.11
        assert_eq!(min, -11);
        assert_eq!(max, 111);
    }

    #[test]
    fn test_add_y_padding_negative_range() {
        let (min, max) = add_y_padding(-100, 0, 10);
        // Same padding calculation but with negative minimum
        assert_eq!(min, -111);
        assert_eq!(max, 11);
    }

    #[test]
    fn test_add_y_padding_mixed_range() {
        let (min, max) = add_y_padding(-50, 50, 10);
        // Range is 100, padding should be ≈ 11.11 on each side
        assert_eq!(min, -61);
        assert_eq!(max, 61);
    }

    #[test]
    fn test_add_y_padding_zero_range() {
        let (min, max) = add_y_padding(0, 0, 10);
        // With zero range, should still add some padding
        assert_eq!(min, 0);
        assert_eq!(max, 0);
    }

    #[test]
    fn test_add_y_padding_large_numbers() {
        let (min, max) = add_y_padding(0, 1_000_000, 10);
        // Range is 1,000,000, padding should be ≈ 111,111
        assert_eq!(min, -111_111);
        assert_eq!(max, 1_111_111);
    }

    #[test]
    fn test_add_y_padding_small_range() {
        let (min, max) = add_y_padding(0, 1, 10);
        // Even with small range, should still add proportional padding
        assert_eq!(min, 0);
        assert_eq!(max, 1);
    }

    #[test]
    fn test_add_y_padding_different_percentages() {
        // Test with 20% padding
        let (min, max) = add_y_padding(0, 100, 20);
        // For 20% padding: range is 100, padding should be (100 * 20/80) = 25
        assert_eq!(min, -25);
        assert_eq!(max, 125);

        // Test with 50% padding
        let (min, max) = add_y_padding(0, 100, 50);
        // For 50% padding: range is 100, padding should be (100 * 50/50) = 100
        assert_eq!(min, -100);
        assert_eq!(max, 200);
    }

    #[test]
    fn test_add_y_padding_max_percentage() {
        let (min, max) = add_y_padding(0, 100, 90);
        // At 90% padding: range is 100, padding should be (100 * 90/10) = 900
        assert_eq!(min, -900);
        assert_eq!(max, 1000);
    }

    #[test]
    fn test_add_y_padding_min_percentage() {
        let (min, max) = add_y_padding(0, 100, 1);
        // At 1% padding: range is 100, padding should be (100 * 1/99) ≈ 1.01
        assert_eq!(min, -1);
        assert_eq!(max, 101);
    }

    #[test]
    fn test_add_y_padding_asymmetric_range() {
        let (min, max) = add_y_padding(75, 100, 10);
        // Range is 25, padding should be (25 * 10/90) ≈ 2.78
        assert_eq!(min, 73);
        assert_eq!(max, 102);
    }

    #[test]
    fn test_add_y_padding_large_range_with_large_offset() {
        let (min, max) = add_y_padding(1_000_000, 1_001_000, 10);
        // Range is 1000, padding should be (1000 * 10/90) ≈ 111
        assert_eq!(min, 999_889);
        assert_eq!(max, 1_001_111);
    }

    #[test]
    fn test_empty_data() {
        let data: Vec<PlotDataPoint<i64>> = vec![];
        let result = calculate_y_min_max(&data);
        assert_eq!(result, None);
    }

    #[test]
    fn test_single_value() {
        let data = vec![PlotDataPoint::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            Some(5),
            None,
            color::GREEN,
        )];
        let result = calculate_y_min_max(&data);
        assert_eq!(result, Some((5, 5)));
    }

    #[test]
    fn test_single_goal() {
        let data = vec![PlotDataPoint::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            None,
            Some(10),
            color::GREEN,
        )];
        let result = calculate_y_min_max(&data);
        assert_eq!(result, Some((10, 10)));
    }

    #[test]
    fn test_all_none() {
        let data = vec![PlotDataPoint::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            None,
            None,
            color::GREEN,
        )];
        let result = calculate_y_min_max(&data);
        assert_eq!(result, None);
    }

    #[test]
    fn test_mixed_values() {
        let data = vec![
            PlotDataPoint::new(
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                Some(5),
                Some(10),
                color::GREEN,
            ),
            PlotDataPoint::new(
                NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
                Some(-3),
                None,
                color::GREEN,
            ),
            PlotDataPoint::new(
                NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
                None,
                Some(7),
                color::GREEN,
            ),
        ];
        let result = calculate_y_min_max(&data);
        assert_eq!(result, Some((-3, 10)));
    }

    #[test]
    fn test_negative_values() {
        let data = vec![PlotDataPoint::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            Some(-10),
            Some(-5),
            color::GREEN,
        )];
        let result = calculate_y_min_max(&data);
        assert_eq!(result, Some((-10, -5)));
    }

    #[test]
    fn test_same_values() {
        let data = vec![PlotDataPoint::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            Some(5),
            Some(5),
            color::GREEN,
        )];
        let result = calculate_y_min_max(&data);
        assert_eq!(result, Some((5, 5)));
    }
}
