// use chrono::{Duration, NaiveDate, Timelike};
// use crate::plot::plot_data::{TimePointPlotData, TimePointPlotPoint};
// use plotters::prelude::*;
// use anyhow::Result;
// use crate::util::color_consts::{BACKGROUND_COLOR, GOAL_LINE_COLOR};
// use crate::util::axis_label_formatter::{seconds_to_time_formatter};
// use crate::plotters_backend::common::get_border;
// use crate::util::tick_calculation::calculate_seconds_ticks;
//
// const PLOT_HEIGHT: u32 = 200;
// const PLOT_WIDTH: u32 = 450;
//
// pub fn plot_time_points(
//     save_path: &std::path::Path,
//     title: &str,
//     date_start: NaiveDate,
//     date_end: NaiveDate,
//     data: TimePointPlotData,
// ) -> Result<()> {
//     let root = BitMapBackend::new(&save_path, (PLOT_WIDTH, PLOT_HEIGHT)).into_drawing_area();
//     root.fill(BACKGROUND_COLOR)?;
//
//     let (y_min, y_max) = calculate_y_range(&data.data_points);
//
//     let y_padding = Duration::minutes(30).num_seconds() as i64;
//     let y_min = (y_min - y_padding).max(0);
//     let y_max = (y_max + y_padding).min(24 * 60 * 60 - 1);
//
//     let mut chart = ChartBuilder::on(&root)
//         .x_label_area_size(25)
//         .y_label_area_size(52)
//         .margin_right(7)
//         .caption(title, ("IPAMincho", 20).into_font().color(&WHITE))
//         .build_cartesian_2d(
//             (date_start..date_end).into_segmented(),
//             (y_min..y_max)
//                 .with_key_points(calculate_seconds_ticks(
//                     y_min,
//                     y_max,
//                     PLOT_HEIGHT,
//                 )),
//         )?;
//
//     chart
//         .configure_mesh()
//         .disable_x_mesh()
//         .bold_line_style(WHITE.mix(0.2))
//         .x_label_formatter(&|seg| match seg {
//             SegmentValue::Exact(d) => d.format("%d.%m").to_string(),
//             SegmentValue::CenterOf(d) => d.format("%d.%m").to_string(),
//             SegmentValue::Last => "".to_string(),
//         })
//         .y_label_formatter(&seconds_to_time_formatter)
//         .axis_style(data.border_color)
//         .label_style(("IPAMincho", 20, &WHITE))
//         .draw()?;
//
//     // Draw horizontal lines for each data point
//     chart.draw_series(data.data_points.iter().map(|point| {
//         PathElement::new(
//             vec![
//                 (
//                     SegmentValue::Exact(point.date),
//                     point.time.num_seconds_from_midnight() as i64,
//                 ),
//                 (
//                     SegmentValue::Exact(point.date + Duration::days(1)),
//                     point.time.num_seconds_from_midnight() as i64,
//                 ),
//             ],
//             point.color.stroke_width(2),
//         )
//     }))?;
//
//     // Draw goal lines
//     for point in &data.data_points {
//         if let Some(goal) = point.goal {
//             chart.draw_series(LineSeries::new(
//                 vec![
//                     (
//                         SegmentValue::Exact(point.date),
//                         goal.num_seconds_from_midnight() as i64,
//                     ),
//                     (
//                         SegmentValue::Exact(point.date + Duration::days(1)),
//                         goal.num_seconds_from_midnight() as i64,
//                     ),
//                 ],
//                 GOAL_LINE_COLOR.mix(0.7).stroke_width(2),
//             ))?;
//         }
//     }
//
//     chart.draw_series([
//         get_border(
//             &data.border_color,
//             date_start,
//             date_end,
//             y_max,
//         )])?;
//
//     root.present()?;
//     Ok(())
// }
//
// fn calculate_y_range(data_points: &[TimePointPlotPoint]) -> (i64, i64) {
//     let mut min_time = 24 * 60 * 60 - 1; // Initialize to end of day
//     let mut max_time = 0;
//
//     for point in data_points {
//         let time_seconds = point.time.num_seconds_from_midnight() as i64;
//         min_time = min_time.min(time_seconds);
//         max_time = max_time.max(time_seconds);
//
//         if let Some(goal) = point.goal {
//             let goal_seconds = goal.num_seconds_from_midnight() as i64;
//             min_time = min_time.min(goal_seconds);
//             max_time = max_time.max(goal_seconds);
//         }
//     }
//
//     (min_time, max_time)
// }