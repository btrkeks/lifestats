use chrono::{Duration, NaiveDate};
use plotters::prelude::*;
use crate::backends::plotters::constants::GOAL_LINE_COLOR;
use crate::objects;
use crate::objects::color::Color::RGB;

pub fn into_goal_lines(values: &[(NaiveDate, i64)]) -> Vec<PathElement<(SegmentValue<NaiveDate>, i64)>> {
    values.iter()
        .map(|(date, goal)| {
            PathElement::new(
                vec![
                    (SegmentValue::Exact(*date), *goal),
                    (SegmentValue::Exact(*date + Duration::days(1)), *goal),
                ],
                GOAL_LINE_COLOR.mix(0.7).stroke_width(2),
            )
        })
        .collect()
}

pub fn get_bounds(points: &[(NaiveDate, i64)]) -> (NaiveDate, NaiveDate, i64, i64) {
    if points.is_empty() {
        return (NaiveDate::default(), NaiveDate::default(), 0, 0);
    }

    let mut x_min = points[0].0;
    let mut x_max = points[0].0;
    let mut y_min = points[0].1;
    let mut y_max = points[0].1;

    for &(x, y) in points.iter() {
        if x < x_min {
            x_min = x;
        }
        if x > x_max {
            x_max = x;
        }
        if y < y_min {
            y_min = y;
        }
        if y > y_max {
            y_max = y;
        }
    }

    (x_min, x_max, y_min, y_max)
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
    color: &RGBColor,
) -> Rectangle<(SegmentValue<NaiveDate>, i64)> {
    let x0 = SegmentValue::Exact(date);
    let x1 = SegmentValue::Exact(date + Duration::days(1));
    let mut bar = Rectangle::new([(x0, y0), (x1, y1)], color.filled());
    bar.set_margin(0, 0, 2, 2);
    bar
}

pub fn color_to_plotters(color: objects::color::Color) -> RGBColor {
    let (r,g,b) = color.into_rgb();
    RGBColor(r,g,b)
}