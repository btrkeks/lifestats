use crate::color_consts::BACKGROUND_COLOR;
use crate::plot_data::{BarplotData, BarplotDataPoint};
use crate::plotters_backend::util::{draw_border, get_bar};
use anyhow::Result;
use backend::paths::get_plot_path;
use chrono::{Duration, NaiveDate};
use plotters::coord::combinators::WithKeyPoints;
use plotters::coord::ranged1d::SegmentedCoord;
use plotters::coord::types::RangedCoordi64;
use plotters::prelude::*;

const GOAL_LINE_COLOR: RGBColor = RED;

// ----------------------------------------------------------------------------------------------

fn draw_goal_lines(
    chart: &mut ChartContext<
        BitMapBackend,
        Cartesian2d<SegmentedCoord<RangedDate<NaiveDate>>, WithKeyPoints<RangedCoordi64>>,
    >,
    data: &[BarplotDataPoint],
) -> Result<()> {
    for (date, goal) in data.iter().map(|x| (x.date, x.goal)) {
        if let Some(goal) = goal {
            chart.draw_series(LineSeries::new(
                vec![
                    (SegmentValue::Exact(date), goal),
                    (SegmentValue::Exact(date + Duration::days(1)), goal),
                ],
                GOAL_LINE_COLOR.mix(0.7).stroke_width(2),
            ))?;
        }
    }
    Ok(())
}

fn get_y_max(data: &[BarplotDataPoint]) -> i64 {
    // TODO also consider lines
    data.iter().map(|row| row.value).max().unwrap_or_default()
}

pub fn plot_barplot(
    filename: &str,
    title: &str,
    date_start: NaiveDate,
    date_end: NaiveDate,
    data: BarplotData,
) -> Result<()> {
    const PLOT_HEIGHT: u32 = 200;
    let plot_path = get_plot_path()?.join(filename);
    let root = BitMapBackend::new(&plot_path, (450, PLOT_HEIGHT)).into_drawing_area();

    root.fill(BACKGROUND_COLOR)?;

    let y_max = get_y_max(&data.data_points);

    let y_margin = (0.1 * (y_max as f64)) as i64;

    let x_axis_min = date_start;
    let x_axis_max = date_end;
    let y_axis_max = (y_max + y_margin).max(100);

    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(25)
        .y_label_area_size(52)
        .margin_right(7)
        .caption(title, ("IPAMincho", 20).into_font().color(&WHITE))
        .build_cartesian_2d(
            (x_axis_min..x_axis_max).into_segmented(),
            (0..y_axis_max).with_key_points(data.get_y_ticks(0, y_axis_max, PLOT_HEIGHT)),
        )?;

    chart
        .configure_mesh()
        .disable_x_mesh()
        .bold_line_style(WHITE.mix(0.2))
        .x_label_formatter(&|seg| match seg {
            SegmentValue::Exact(d) => d.format("%d.%m").to_string(),
            SegmentValue::CenterOf(d) => d.format("%d.%m").to_string(),
            SegmentValue::Last => "".to_string(),
        })
        .y_label_formatter(&data.get_y_formatter())
        .axis_style(data.border_color)
        .label_style(("IPAMincho", 20, &WHITE))
        .draw()?;

    chart.draw_series(
        data.data_points
            .iter()
            .map(|point| get_bar(point.date, 0, point.value, &point.color)),
    )?;

    draw_goal_lines(&mut chart, &data.data_points)?;
    draw_border(
        &data.border_color,
        x_axis_min,
        x_axis_max,
        y_axis_max,
        &mut chart,
    )?;

    root.present()?;
    Ok(())
}
