// use crate::util::color_consts::{BACKGROUND_COLOR, GOAL_LINE_COLOR};
// use crate::plot::plot_data::{BarplotData, BarplotDataPoint};
// use crate::plotters_backend::common::{get_border, get_bar};
// use anyhow::Result;
// use chrono::{Duration, NaiveDate};
// use plotters::prelude::*;
//
// const PLOT_WIDTH: u32 = 450;
// const PLOT_HEIGHT: u32 = 200;
// const Y_MARGIN_PERCENT: i64 = 10;
//
// fn get_goal_lines(
//     data: &[BarplotDataPoint],
// ) -> Vec<PathElement<(SegmentValue<NaiveDate>, i64)>> {
//     data.iter()
//         .filter_map(|data_point| {
//             if let Some(goal) = data_point.goal {
//                 Some((data_point.date, goal))
//             }
//             else {
//                 None
//             }
//         })
//         .map(|(date, goal)| {
//             PathElement::new(
//                 vec![
//                     (SegmentValue::Exact(date), goal),
//                     (SegmentValue::Exact(date + Duration::days(1)), goal),
//                 ],
//                 GOAL_LINE_COLOR.mix(0.7).stroke_width(2),
//             )
//         })
//         .collect()
// }
//
// fn get_y_max(data: &[BarplotDataPoint]) -> i64 {
//     data.iter()
//         .flat_map(|point| [point.value, point.goal.unwrap_or(0)])
//         .max()
//         .unwrap_or(0)
// }
//
// pub fn plot_barplot(
//     save_path: &std::path::Path,
//     title: &str,
//     date_start: NaiveDate,
//     date_end: NaiveDate,
//     data: &BarplotData,
// ) -> Result<()> {
//     let root = BitMapBackend::new(&save_path, (PLOT_WIDTH, PLOT_HEIGHT)).into_drawing_area();
//     root.fill(BACKGROUND_COLOR)?;
//
//     let y_max = get_y_max(&data.data_points);
//     let y_margin = Y_MARGIN_PERCENT * y_max / 100;
//     let y_axis_max = (y_max + y_margin).max(100);
//
//     let x_axis_min = date_start;
//     let x_axis_max = date_end;
//
//     let mut chart = ChartBuilder::on(&root)
//         .x_label_area_size(25)
//         .y_label_area_size(52)
//         .margin_right(7)
//         .caption(title, ("IPAMincho", 20).into_font().color(&WHITE))
//         .build_cartesian_2d(
//             (x_axis_min..x_axis_max).into_segmented(),
//             (0..y_axis_max).with_key_points(data.get_y_ticks(0, y_axis_max, PLOT_HEIGHT)),
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
//         .y_label_formatter(&data.get_y_formatter())
//         .axis_style(data.border_color)
//         .label_style(("IPAMincho", 20, &WHITE))
//         .draw()?;
//
//     chart.draw_series(
//         data.data_points
//             .iter()
//             .map(|point| get_bar(point.date, 0, point.value, &point.color)),
//     )?;
//
//     chart.draw_series(get_goal_lines(&data.data_points))?;
//
//     chart.draw_series([
//         get_border(
//             &data.border_color,
//             x_axis_min,
//             x_axis_max,
//             y_axis_max,
//         )]
//     )?;
//
//     root.present()?;
//     Ok(())
// }
