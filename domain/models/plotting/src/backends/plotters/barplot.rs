use crate::objects::image::Image;
use crate::objects::plot_data::{PlotDataPoint};
use anyhow::Result;
use chrono::NaiveDate;
use plotters::prelude::*;
use crate::backends::plotters::common::{color_to_plotters, get_bar, get_border, get_bounds, into_goal_lines};
use crate::backends::plotters::constants::{BACKGROUND_COLOR, Y_MARGIN_PERCENT};
use crate::objects;
use crate::objects::y_axis_conversion::YAxisConversion;

pub fn barplot(title: String,
               size: (usize, usize),
               data: Vec<PlotDataPoint<i64>>,
               y_unit: String,
               border_color: RGBColor,
               y_axis_conversion: YAxisConversion,
               x_min: NaiveDate,
               x_max: NaiveDate,
) -> Result<Image> {
    let mut image = Image::new_with_size(size);
    {
        let root = BitMapBackend::with_buffer(image.get_buffer(),
                                              (size.0 as u32, size.1 as u32)).into_drawing_area();
        root.fill(BACKGROUND_COLOR)?;

        let y_axis_max = calculate_y_axis_max(&data);

        let mut chart = ChartBuilder::on(&root)
            .x_label_area_size(25)
            .y_label_area_size(52)
            .margin_right(7)
            .caption(title, ("IPAMincho", 20).into_font().color(&WHITE))
            .build_cartesian_2d(
                (x_min..x_max).into_segmented(),
                (0..y_axis_max).with_key_points(y_axis_conversion.get_ticks(0, y_axis_max, size.1)),
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
            .y_label_formatter(y_axis_conversion.get_formatter())
            .axis_style(border_color)
            .label_style(("IPAMincho", 20, &WHITE))
            .draw()?;

        // Draw bars
        chart.draw_series(
            data.iter()
                .map(|point|
                    get_bar(point.date, 0, point.value, &color_to_plotters(point.color))
                ),
        )?;

        // Draw goal lines
        // TODO: More efficient?
        let goals: Vec<(NaiveDate, i64)> = data.iter()
            .filter_map(|point| {
                if let Some(goal) = point.goal {
                    Some((point.date, goal))
                } else {
                    None
                }
            })
            .collect();
        chart.draw_series(into_goal_lines(&goals))?;

        // Draw border
        chart.draw_series([
            get_border(
                &border_color,
                x_min,
                x_max,
                y_axis_max,
            )]
        )?;

        root.present()?;
    }
    Ok(image)
}

fn calculate_y_axis_max(data: &Vec<PlotDataPoint<i64>>) -> i64 {
    let raw_data: Vec<(NaiveDate, i64)> = data.iter()
        .map(|point| (point.date, point.value))
        .collect();

    let (_, _, _, y_max) = get_bounds(&raw_data); // TODO: Bit inefficient
    let y_margin = Y_MARGIN_PERCENT * y_max / 100;
    let y_axis_max = y_max + y_margin;

    y_axis_max
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_barplot_basic_functionality() -> Result<()> {
        // Test data setup
        let test_data = vec![
            PlotDataPoint {
                date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                value: 100,
                goal: Some(120),
                color: objects::color::Color::RGB(255, 0, 0),
            },
            PlotDataPoint {
                date: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
                value: 150,
                goal: Some(120),
                color: objects::color::Color::RGB(0, 255, 0),
            },
            PlotDataPoint {
                date: NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
                value: 80,
                goal: Some(120),
                color: objects::color::Color::RGB(0, 0, 255),
            },
        ];

        let title = String::from("Test Bar Plot");
        let size = (800, 600);
        let y_unit = String::from("units");
        let border_color = RGBColor(255, 255, 255);
        let y_axis_conversion = YAxisConversion::None;

        // Generate the plot
        let result = barplot(
            title,
            size,
            test_data,
            y_unit,
            border_color,
            y_axis_conversion,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 1, 5).unwrap()
        )?;

        // Save the plot to a temporary file for verification
        let temp_path = std::path::Path::new("test_barplot.png");
        result.save(&temp_path)?;

        // Verify the file exists and has non-zero size
        assert!(temp_path.exists());
        assert!(temp_path.metadata()?.len() > 0);

        // Clean up
        std::fs::remove_file(temp_path)?;

        Ok(())
    }

    // #[test]
    // fn test_barplot_empty_data() -> Result<()> {
    //     let empty_data = vec![];
    //     let title = String::from("Empty Bar Plot");
    //     let size = (800, 600);
    //     let y_unit = String::from("units");
    //     let border_color = RGBColor(255, 255, 255);
    //     let y_axis_conversion = YAxisConversion::None;
    //
    //     // This should return an error since we can't plot empty data
    //     let result = barplot(
    //         title,
    //         size,
    //         empty_data,
    //         y_unit,
    //         border_color,
    //         y_axis_conversion,
    //     );
    //
    //     assert!(result.is_err());
    //     Ok(())
    // }

    // #[test]
    // fn test_barplot_size_validation() -> Result<()> {
    //     let test_data = vec![
    //         PlotDataPoint {
    //             date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    //             value: 100,
    //             goal: None,
    //             color: RGBColor::new(255, 0, 0),
    //         },
    //     ];
    //
    //     // Test with zero dimensions
    //     let result = barplot(
    //         String::from("Zero Size Plot"),
    //         (0, 0),
    //         test_data.clone(),
    //         String::from("units"),
    //         RGBColor::new(255, 255, 255),
    //         YAxisConversion::default(),
    //     );
    //     assert!(result.is_err());
    //
    //     // Test with very small dimensions
    //     let result = barplot(
    //         String::from("Small Size Plot"),
    //         (10, 10),
    //         test_data,
    //         String::from("units"),
    //         RGBColor::new(255, 255, 255),
    //         YAxisConversion::default(),
    //     );
    //     assert!(result.is_err());
    //
    //     Ok(())
    // }
    //
    // #[test]
    // fn test_barplot_axis_bounds() -> Result<()> {
    //     let test_data = vec![
    //         PlotDataPoint {
    //             date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    //             value: 100,
    //             goal: Some(150),
    //             color: RGBColor::new(255, 0, 0),
    //         },
    //         PlotDataPoint {
    //             date: NaiveDate::from_ymd_opt(2024, 1, 5).unwrap(),
    //             value: 200,
    //             goal: Some(150),
    //             color: RGBColor::new(0, 255, 0),
    //         },
    //     ];
    //
    //     // Test the axis bounds calculation
    //     let (x_min, x_max, y_max) = get_axis_bounds(&test_data);
    //
    //     assert_eq!(x_min, NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    //     assert_eq!(x_max, NaiveDate::from_ymd_opt(2024, 1, 5).unwrap());
    //
    //     // Check if y_max includes the margin
    //     let expected_y_max = (200.0 * (1.0 + Y_MARGIN_PERCENT as f64 / 100.0)) as i64;
    //     assert_eq!(y_max, expected_y_max);
    //
    //     Ok(())
    // }
    //
    // #[test]
    // fn test_barplot_with_large_dataset() -> Result<()> {
    //     // Generate a large dataset
    //     let mut test_data = Vec::new();
    //     for i in 0..100 {
    //         test_data.push(PlotDataPoint {
    //             date: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap() + chrono::Duration::days(i),
    //             value: (i * 10) as i64,
    //             goal: Some(500),
    //             color: RGBColor::new(255, 0, 0),
    //         });
    //     }
    //
    //     let result = barplot(
    //         String::from("Large Dataset Plot"),
    //         (1200, 800),
    //         test_data,
    //         String::from("units"),
    //         RGBColor::new(255, 255, 255),
    //         YAxisConversion::default(),
    //     )?;
    //
    //     let temp_path = Path::new("test_large_barplot.png");
    //     result.save(&temp_path)?;
    //
    //     assert!(temp_path.exists());
    //     assert!(temp_path.metadata()?.len() > 0);
    //
    //     std::fs::remove_file(temp_path)?;
    //
    //     Ok(())
    // }
}