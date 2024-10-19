use anyhow::{Context, Result};
use chrono::{Duration, NaiveDate};
use plotters::coord::combinators::WithKeyPoints;
use plotters::coord::ranged1d::SegmentedCoord;
use plotters::coord::types::{RangedCoordf32, RangedCoordi64};
use plotters::prelude::*;

pub fn draw_border(
    border_color: &RGBColor,
    x_axis_min: NaiveDate,
    x_axis_max: NaiveDate,
    y_axis_max: i64,
    chart: &mut ChartContext<
        BitMapBackend,
        Cartesian2d<SegmentedCoord<RangedDate<NaiveDate>>, WithKeyPoints<RangedCoordi64>>,
    >,
) -> Result<()> {
    chart.plotting_area().draw(&Rectangle::new(
        [
            (SegmentValue::Exact(x_axis_min), 0),
            (
                SegmentValue::Exact(x_axis_max + Duration::days(1)),
                y_axis_max,
            ),
        ],
        border_color.stroke_width(1),
    ))?;
    Ok(())
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

pub fn draw_line(
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

pub fn draw_gray_dashed_line(
    cc: &mut ChartContext<
        BitMapBackend,
        Cartesian2d<RangedCoordf32, WithKeyPoints<RangedCoordi64>>,
    >,
    y_value: i64,
) -> Result<()> {
    let x_range = cc.x_range();
    cc.draw_series(DashedLineSeries::new(
        vec![(x_range.start, y_value), (x_range.end, y_value)],
        5,
        10,
        ShapeStyle {
            color: RGBColor(128, 128, 128).mix(0.7),
            filled: false,
            stroke_width: 1,
        },
    ))
    .context("Drawing dashed line")?;
    Ok(())
}
