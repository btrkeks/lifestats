// use plotters::prelude::*;
//
// pub(crate) fn draw_circular_progress(current: f64, max: f64) -> Result<(), Box<dyn std::error::Error>> {
//     let out_file = "circular_progress.png";
//     let root = BitMapBackend::new(out_file, (500, 500)).into_drawing_area();
//     root.fill(&WHITE)?;
//
//     let mut chart = ChartBuilder::on(&root)
//         .build_cartesian_2d(-1f64..1f64, -1f64..1f64)?;
//
//     chart.configure_mesh().disable_mesh().draw()?;
//
//     let progress = current / max;
//
//     // Draw background thin ring
//     chart.draw_series(std::iter::once(Circle::new(
//         (0.0, 0.0),
//         150,
//         ShapeStyle::from(&RGBColor(200, 200, 200)).stroke_width(10),
//     )))?;
//
//     // Draw progress arc using multiple small circles
//     let start_angle = std::f64::consts::PI / 2.0;
//     let end_angle = 2.0 * std::f64::consts::PI * progress + start_angle;
//     let num_circles = 200;
//     let radius = 0.6;
//     let blue_style = ShapeStyle::from(&BLUE).filled();
//
//     for i in 0..=num_circles {
//         let angle = (i as f64 / num_circles as f64) * end_angle + start_angle;
//         let x = radius * angle.cos();
//         let y = radius * angle.sin();
//
//         if angle <= end_angle {
//             chart.draw_series(std::iter::once(Circle::new(
//                 (x, y),
//                 4,
//                 blue_style.clone(),
//             )))?;
//         }
//     }
//
//     // Draw percentage text
//     let percentage = (progress * 100.0).round() as i32;
//     chart.draw_series(std::iter::once(Text::new(
//         format!("{current}/{max}"),
//         (0.0, 0.0),
//         ("sans-serif", 30.0).into_font().color(&BLACK),
//     )))?;
//
//     root.present()?;
//     println!("Chart has been saved to {}", out_file);
//
//     Ok(())
// }
