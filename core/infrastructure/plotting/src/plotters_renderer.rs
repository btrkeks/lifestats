use anyhow::{anyhow, Result};
use chrono::Timelike;
use image::{ImageBuffer, Rgb};
use plot_domain::plot::axis::{AxisLabelFormat, YAxis};
use plot_domain::plot::data::{DrawableData, DrawableGoal, PlotData};
use plot_domain::plot::plot_type::PlotType;
use plot_domain::plot::renderer::{PlotRenderer, RenderConfig};
use plotters::prelude::*;

use crate::common::{
    calculate_seconds_ticks, duration_seconds_to_hh_mm, get_bar, get_border, get_horizontal_line,
    number_formatter, percent_formatter, time_of_day_seconds_to_hh_mm, BACKGROUND_COLOR,
    GOAL_LINE_COLOR, MISSED_EMPTY_GOAL_DAY_COLOR, SHADING_COLOR,
};

const MISSED_GOAL_COLOR: [u8; 3] = [245, 169, 142];

/// Concrete implementation of PlotRenderer using the plotters library
pub struct PlottersRenderer;

impl PlottersRenderer {
    pub fn new() -> Self {
        Self
    }

    fn scalar_data_value(data: &DrawableData) -> Option<i64> {
        match data {
            DrawableData::Scalar(value) => Some(value.value()),
            DrawableData::TimeFrames(_) => None,
        }
    }

    fn scalar_goal_value(goal: &DrawableGoal) -> Option<i64> {
        match goal {
            DrawableGoal::Scalar(value) => Some(value.value()),
            DrawableGoal::TimeFrames(_) => None,
        }
    }

    fn scalar_axis_bounds(axis: &YAxis, _plot_name: &str) -> Result<(i64, i64, AxisLabelFormat)> {
        match axis {
            YAxis::Scalar {
                min,
                max,
                label_format,
            } => Ok((*min, *max, *label_format)),
            YAxis::TimeOfDay {
                min_seconds,
                max_seconds,
                label_format,
            } => Ok((*min_seconds, *max_seconds, *label_format)),
        }
    }

    fn format_y_label(label_format: AxisLabelFormat, value: &i64) -> String {
        match label_format {
            AxisLabelFormat::Number => number_formatter(value),
            AxisLabelFormat::DurationHhMm => duration_seconds_to_hh_mm(value),
            AxisLabelFormat::TimeOfDayHhMm => time_of_day_seconds_to_hh_mm(value),
            AxisLabelFormat::Percent => percent_formatter(value),
        }
    }

    fn should_shade_empty_missed_goal_day(point: &plot_domain::plot::data::DataPoint) -> bool {
        if point.goal.is_none() || point.color != MISSED_GOAL_COLOR {
            return false;
        }

        match &point.value {
            None => true,
            Some(DrawableData::Scalar(value)) => value.value() == 0,
            _ => false,
        }
    }

    fn render_bar_chart(&self, data: &PlotData, config: &RenderConfig) -> Result<Vec<u8>> {
        let mut buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(config.width, config.height);
        let root = BitMapBackend::with_buffer(&mut buffer, (config.width, config.height))
            .into_drawing_area();

        root.fill(&BACKGROUND_COLOR)?;

        let (y_axis_min, y_axis_max, label_format) =
            Self::scalar_axis_bounds(&data.y_axis, "BarChart")?;

        let mut chart = ChartBuilder::on(&root)
            .x_label_area_size(25)
            .y_label_area_size(52)
            .margin_right(7)
            .caption(&data.title, ("IPAMincho", 20).into_font().color(&WHITE))
            .build_cartesian_2d(
                (data.date_range.start()..data.date_range.end()).into_segmented(),
                y_axis_min..y_axis_max,
            )?;

        chart
            .configure_mesh()
            .disable_x_mesh()
            .x_labels((data.date_range.end() - data.date_range.start()).num_days() as usize + 1)
            .bold_line_style(WHITE.mix(0.2))
            .x_label_formatter(&|seg| match seg {
                SegmentValue::Exact(d) => d.format("%d.%m").to_string(),
                SegmentValue::CenterOf(d) => d.format("%d.").to_string(),
                SegmentValue::Last => "".to_string(),
            })
            .y_label_formatter(&|value| Self::format_y_label(label_format, value))
            .axis_style(WHITE)
            .label_style(("IPAMincho", 20, &WHITE))
            .draw()?;

        // Mark goal days where the missed value would otherwise be invisible.
        for point in &data.points {
            if Self::should_shade_empty_missed_goal_day(point) {
                chart.draw_series([get_bar(
                    point.date,
                    0,
                    y_axis_max,
                    0,
                    &MISSED_EMPTY_GOAL_DAY_COLOR,
                )])?;
            }
        }

        // Draw bars using DrawableData and pre-calculated colors with helper function
        for point in &data.points {
            if let Some(value) = point.value.as_ref().and_then(Self::scalar_data_value) {
                let [r, g, b] = point.color;
                let color = RGBColor(r, g, b);
                chart.draw_series([get_bar(point.date, 0, value, 2, &color.to_rgba())])?;
            }
        }

        // Draw goal lines using helper function
        for point in &data.points {
            if let Some(goal_number) = point.goal.as_ref().and_then(Self::scalar_goal_value) {
                chart.draw_series([get_horizontal_line(
                    point.date,
                    goal_number,
                    GOAL_LINE_COLOR,
                )])?;
            }
        }

        // Draw border
        chart.draw_series([get_border(
            &WHITE,
            data.date_range.start(),
            data.date_range.end(),
            y_axis_max,
        )])?;

        drop(chart);
        drop(root);

        // Encode to PNG
        let mut png_buffer = Vec::new();
        image::DynamicImage::ImageRgb8(buffer).write_to(
            &mut std::io::Cursor::new(&mut png_buffer),
            image::ImageFormat::Png,
        )?;

        Ok(png_buffer)
    }

    fn render_line_chart(&self, data: &PlotData, config: &RenderConfig) -> Result<Vec<u8>> {
        let mut buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(config.width, config.height);
        let root = BitMapBackend::with_buffer(&mut buffer, (config.width, config.height))
            .into_drawing_area();

        root.fill(&BACKGROUND_COLOR)?;

        let (y_axis_min, y_axis_max, label_format) =
            Self::scalar_axis_bounds(&data.y_axis, "LineChart")?;

        let mut chart = ChartBuilder::on(&root)
            .x_label_area_size(25)
            .y_label_area_size(52)
            .margin_right(7)
            .caption(&data.title, ("IPAMincho", 20).into_font().color(&WHITE))
            .build_cartesian_2d(
                (data.date_range.start()..data.date_range.end()).into_segmented(),
                y_axis_min..y_axis_max,
            )?;

        chart
            .configure_mesh()
            .disable_x_mesh()
            .x_labels((data.date_range.end() - data.date_range.start()).num_days() as usize + 1)
            .bold_line_style(WHITE.mix(0.2))
            .x_label_formatter(&|seg| match seg {
                SegmentValue::Exact(d) => d.format("%d.%m").to_string(),
                SegmentValue::CenterOf(d) => d.format("%d.").to_string(),
                SegmentValue::Last => "".to_string(),
            })
            .y_label_formatter(&|value| Self::format_y_label(label_format, value))
            .axis_style(WHITE)
            .label_style(("IPAMincho", 20, &WHITE))
            .draw()?;

        // Draw line connecting points
        let line_data: Vec<_> = data
            .points
            .iter()
            .filter_map(|point| {
                point
                    .value
                    .as_ref()
                    .and_then(Self::scalar_data_value)
                    .map(|value| (SegmentValue::CenterOf(point.date), value))
            })
            .collect();

        if !line_data.is_empty() {
            chart.draw_series(LineSeries::new(line_data, WHITE.mix(0.8)))?;
        }

        // Draw points with pre-calculated colors
        for point in &data.points {
            if let Some(value) = point.value.as_ref().and_then(Self::scalar_data_value) {
                let [r, g, b] = point.color;
                let color = RGBColor(r, g, b);
                chart.draw_series([Circle::new(
                    (SegmentValue::CenterOf(point.date), value),
                    5,
                    color.filled(),
                )])?;
            }
        }

        // Draw goal lines
        for point in &data.points {
            if let Some(goal_number) = point.goal.as_ref().and_then(Self::scalar_goal_value) {
                chart.draw_series([get_horizontal_line(
                    point.date,
                    goal_number,
                    GOAL_LINE_COLOR,
                )])?;
            }
        }

        // Draw border
        chart.draw_series([get_border(
            &WHITE,
            data.date_range.start(),
            data.date_range.end(),
            y_axis_max,
        )])?;

        drop(chart);
        drop(root);

        // Encode to PNG
        let mut png_buffer = Vec::new();
        image::DynamicImage::ImageRgb8(buffer).write_to(
            &mut std::io::Cursor::new(&mut png_buffer),
            image::ImageFormat::Png,
        )?;

        Ok(png_buffer)
    }

    fn render_scatter(&self, data: &PlotData, config: &RenderConfig) -> Result<Vec<u8>> {
        let mut buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(config.width, config.height);
        let root = BitMapBackend::with_buffer(&mut buffer, (config.width, config.height))
            .into_drawing_area();

        root.fill(&BACKGROUND_COLOR)?;

        let (y_axis_min, y_axis_max, label_format) =
            Self::scalar_axis_bounds(&data.y_axis, "Scatter")?;

        let mut chart = ChartBuilder::on(&root)
            .x_label_area_size(25)
            .y_label_area_size(52)
            .margin_right(7)
            .caption(&data.title, ("IPAMincho", 20).into_font().color(&WHITE))
            .build_cartesian_2d(
                (data.date_range.start()..data.date_range.end()).into_segmented(),
                y_axis_min..y_axis_max,
            )?;

        chart
            .configure_mesh()
            .disable_x_mesh()
            .x_labels((data.date_range.end() - data.date_range.start()).num_days() as usize + 1)
            .bold_line_style(WHITE.mix(0.2))
            .x_label_formatter(&|seg| match seg {
                SegmentValue::Exact(d) => d.format("%d.%m").to_string(),
                SegmentValue::CenterOf(d) => d.format("%d.").to_string(),
                SegmentValue::Last => "".to_string(),
            })
            .y_label_formatter(&|value| Self::format_y_label(label_format, value))
            .axis_style(WHITE)
            .label_style(("IPAMincho", 20, &WHITE))
            .draw()?;

        // Draw horizontal lines for each data point (old scatter plot style)
        for point in &data.points {
            if let Some(value) = point.value.as_ref().and_then(Self::scalar_data_value) {
                let [r, g, b] = point.color;
                let color = RGBColor(r, g, b);
                chart.draw_series([get_horizontal_line(point.date, value, color.to_rgba())])?;
            }
        }

        // Draw goal lines
        for point in &data.points {
            if let Some(goal_number) = point.goal.as_ref().and_then(Self::scalar_goal_value) {
                chart.draw_series([get_horizontal_line(
                    point.date,
                    goal_number,
                    GOAL_LINE_COLOR,
                )])?;
            }
        }

        // Draw border
        chart.draw_series([get_border(
            &WHITE,
            data.date_range.start(),
            data.date_range.end(),
            y_axis_max,
        )])?;

        drop(chart);
        drop(root);

        // Encode to PNG
        let mut png_buffer = Vec::new();
        image::DynamicImage::ImageRgb8(buffer).write_to(
            &mut std::io::Cursor::new(&mut png_buffer),
            image::ImageFormat::Png,
        )?;

        Ok(png_buffer)
    }

    fn render_time_frames(&self, data: &PlotData, config: &RenderConfig) -> Result<Vec<u8>> {
        let mut buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(config.width, config.height);
        let root = BitMapBackend::with_buffer(&mut buffer, (config.width, config.height))
            .into_drawing_area();

        root.fill(&BACKGROUND_COLOR)?;

        let (y_axis_min, y_axis_max, label_format) = match &data.y_axis {
            YAxis::TimeOfDay {
                min_seconds,
                max_seconds,
                label_format,
            } => (*min_seconds, *max_seconds, *label_format),
            YAxis::Scalar { .. } => {
                return Err(anyhow!("TimeFrames plot requires TimeOfDay axis"));
            }
        };

        let mut chart = ChartBuilder::on(&root)
            .x_label_area_size(25)
            .y_label_area_size(52)
            .margin_right(7)
            .caption(&data.title, ("IPAMincho", 20).into_font().color(&WHITE))
            .build_cartesian_2d(
                (data.date_range.start()..data.date_range.end()).into_segmented(),
                (y_axis_min..y_axis_max).with_key_points(calculate_seconds_ticks(
                    y_axis_min,
                    y_axis_max,
                    config.height as usize,
                )),
            )?;

        chart
            .configure_mesh()
            .x_labels((data.date_range.end() - data.date_range.start()).num_days() as usize + 1)
            .disable_x_mesh()
            .bold_line_style(WHITE.mix(0.2))
            .x_label_formatter(&|seg| match seg {
                SegmentValue::Exact(d) => d.format("%d.%m").to_string(),
                SegmentValue::CenterOf(d) => d.format("%d.").to_string(),
                SegmentValue::Last => "".to_string(),
            })
            .y_label_formatter(&|value| Self::format_y_label(label_format, value))
            .axis_style(WHITE)
            .label_style(("IPAMincho", 15, &WHITE))
            .draw()?;

        // Draw bars for each data point's time frames
        for point in &data.points {
            // Use pattern matching on DrawableData - type-safe!
            if let Some(DrawableData::TimeFrames(time_frame_list)) = &point.value {
                for time_frame in time_frame_list.iter() {
                    let start_seconds = time_frame.get_start().num_seconds_from_midnight() as i64;
                    let end_seconds = time_frame.get_end().num_seconds_from_midnight() as i64;

                    // Use pre-calculated color from the data point
                    let [r, g, b] = point.color;
                    let color = RGBColor(r, g, b);

                    chart.draw_series([get_bar(
                        point.date,
                        start_seconds,
                        end_seconds,
                        2,
                        &color.to_rgba(),
                    )])?;
                }
            }
        }

        // Draw goal lines and shading
        for point in &data.points {
            // Use pattern matching on DrawableGoal - no more JSON parsing!
            if let Some(DrawableGoal::TimeFrames(goal_time_frames)) = &point.goal {
                if goal_time_frames.is_empty() {
                    continue;
                }

                let mut last_end_time = 0;

                for frame in goal_time_frames.iter() {
                    let current_start_time = frame.get_start().num_seconds_from_midnight() as i64;
                    let current_end_time = frame.get_end().num_seconds_from_midnight() as i64;

                    // Shade the area between the last end time and the current start time
                    chart.draw_series([get_bar(
                        point.date,
                        last_end_time,
                        current_start_time,
                        0,
                        &SHADING_COLOR,
                    )])?;

                    // Draw the goal lines
                    chart.draw_series([
                        get_horizontal_line(
                            point.date,
                            current_start_time,
                            GOAL_LINE_COLOR.mix(0.7),
                        ),
                        get_horizontal_line(point.date, current_end_time, GOAL_LINE_COLOR.mix(0.7)),
                    ])?;

                    last_end_time = current_end_time;
                }

                // Shade the area between the last end time and the y-axis max
                chart.draw_series([get_bar(
                    point.date,
                    last_end_time,
                    y_axis_max,
                    0,
                    &SHADING_COLOR,
                )])?;
            }
        }

        // Draw border
        chart.draw_series([get_border(
            &WHITE,
            data.date_range.start(),
            data.date_range.end(),
            y_axis_max,
        )])?;

        drop(chart);
        drop(root);

        // Encode to PNG
        let mut png_buffer = Vec::new();
        image::DynamicImage::ImageRgb8(buffer).write_to(
            &mut std::io::Cursor::new(&mut png_buffer),
            image::ImageFormat::Png,
        )?;

        Ok(png_buffer)
    }
}

impl Default for PlottersRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl PlotRenderer for PlottersRenderer {
    fn render(&self, data: &PlotData, config: &RenderConfig) -> Result<Vec<u8>> {
        match data.plot_type {
            PlotType::BarChart => self.render_bar_chart(data, config),
            PlotType::LineChart => self.render_line_chart(data, config),
            PlotType::Scatter => self.render_scatter(data, config),
            PlotType::TimeFrames => self.render_time_frames(data, config),
            PlotType::CircularProgress => {
                // TODO: Implement circular progress rendering
                Err(anyhow!("CircularProgress plot type not yet implemented"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use lifestats_core::shared::date_range::DateRange;
    use plot_domain::plot::data::{DataPoint, PlotData, ScalarValue};

    fn test_date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()
    }

    fn point(value: Option<DrawableData>, color: [u8; 3], goal: Option<DrawableGoal>) -> DataPoint {
        DataPoint::new(test_date(), value, color, goal)
    }

    fn number(value: i64) -> DrawableData {
        DrawableData::Scalar(ScalarValue::number(value))
    }

    fn binary(value: bool) -> DrawableData {
        DrawableData::Scalar(ScalarValue::binary(value))
    }

    fn number_goal(value: i64) -> DrawableGoal {
        DrawableGoal::Scalar(ScalarValue::number(value))
    }

    fn binary_goal(value: bool) -> DrawableGoal {
        DrawableGoal::Scalar(ScalarValue::binary(value))
    }

    #[test]
    fn shades_goal_day_with_missing_missed_value() {
        let point = point(None, MISSED_GOAL_COLOR, Some(number_goal(10)));

        assert!(PlottersRenderer::should_shade_empty_missed_goal_day(&point));
    }

    #[test]
    fn shades_goal_day_with_zero_missed_value() {
        let point = point(Some(number(0)), MISSED_GOAL_COLOR, Some(number_goal(10)));

        assert!(PlottersRenderer::should_shade_empty_missed_goal_day(&point));
    }

    #[test]
    fn does_not_shade_nonzero_missed_value() {
        let point = point(Some(number(5)), MISSED_GOAL_COLOR, Some(number_goal(10)));

        assert!(!PlottersRenderer::should_shade_empty_missed_goal_day(
            &point
        ));
    }

    #[test]
    fn does_not_shade_zero_achieved_value() {
        let point = point(Some(number(0)), [169, 252, 177], Some(number_goal(0)));

        assert!(!PlottersRenderer::should_shade_empty_missed_goal_day(
            &point
        ));
    }

    #[test]
    fn does_not_shade_day_without_goal() {
        let point = point(Some(number(0)), MISSED_GOAL_COLOR, None);

        assert!(!PlottersRenderer::should_shade_empty_missed_goal_day(
            &point
        ));
    }

    #[test]
    fn test_render_bar_chart_with_missed_empty_goal_days() {
        let renderer = PlottersRenderer::new();
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 3).unwrap();
        let date_range = DateRange::new(start, end).unwrap();

        let points = vec![
            DataPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                Some(number(12)),
                [169, 252, 177],
                Some(number_goal(10)),
            ),
            DataPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                None,
                MISSED_GOAL_COLOR,
                Some(number_goal(10)),
            ),
            DataPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
                Some(number(0)),
                MISSED_GOAL_COLOR,
                Some(number_goal(10)),
            ),
        ];

        let plot_data = PlotData::new(
            "Missed Empty Goal Days".to_string(),
            points,
            date_range,
            PlotType::BarChart,
            YAxis::Scalar {
                min: 0,
                max: 12,
                label_format: AxisLabelFormat::Number,
            },
        );

        let result = renderer.render(&plot_data, &RenderConfig::default());

        assert!(result.is_ok());
        let png_bytes = result.unwrap();
        assert!(!png_bytes.is_empty());
        assert_eq!(&png_bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_render_binary_bar_chart() {
        let renderer = PlottersRenderer::new();
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
        let date_range = DateRange::new(start, end).unwrap();

        let points = vec![
            DataPoint::new(
                start,
                Some(binary(true)),
                [169, 252, 177],
                Some(binary_goal(true)),
            ),
            DataPoint::new(
                end,
                Some(binary(false)),
                MISSED_GOAL_COLOR,
                Some(binary_goal(true)),
            ),
        ];

        let plot_data = PlotData::new(
            "Habit".to_string(),
            points,
            date_range,
            PlotType::BarChart,
            YAxis::Scalar {
                min: 0,
                max: 1,
                label_format: AxisLabelFormat::Number,
            },
        );

        let result = renderer.render(&plot_data, &RenderConfig::default());

        assert!(result.is_ok());
        let png_bytes = result.unwrap();
        assert!(!png_bytes.is_empty());
        assert_eq!(&png_bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_render_line_chart() {
        let renderer = PlottersRenderer::new();
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
        let date_range = DateRange::new(start, end).unwrap();

        let points = vec![
            DataPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                Some(number(10)),
                [0, 255, 0],
                None,
            ),
            DataPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 5).unwrap(),
                Some(number(20)),
                [0, 255, 0],
                None,
            ),
        ];

        let y_axis = YAxis::Scalar {
            min: 0,
            max: 30,
            label_format: AxisLabelFormat::Number,
        };
        let plot_data = PlotData::new(
            "Test Plot".to_string(),
            points,
            date_range,
            PlotType::LineChart,
            y_axis,
        );

        let config = RenderConfig::default();
        let result = renderer.render(&plot_data, &config);

        assert!(result.is_ok());
        let png_bytes = result.unwrap();
        assert!(!png_bytes.is_empty());

        // Verify it's a valid PNG by checking magic bytes
        assert_eq!(&png_bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_render_time_frames_plot() {
        use lifestats_core::shared::time_frame_list::TimeFrameList;

        let renderer = PlottersRenderer::new();
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 3).unwrap();
        let date_range = DateRange::new(start, end).unwrap();

        // Create TimeFrameList data
        let time_frame_list = serde_json::from_value::<TimeFrameList>(serde_json::json!([
            {"start": "09:00:00", "end": "12:00:00"},
            {"start": "14:00:00", "end": "17:00:00"}
        ]))
        .unwrap();

        let goal_time_frames = serde_json::from_value::<TimeFrameList>(serde_json::json!([
            {"start": "08:00:00", "end": "18:00:00"}
        ]))
        .unwrap();

        let points = vec![
            DataPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                Some(DrawableData::TimeFrames(time_frame_list.clone())),
                [0, 255, 0],
                Some(DrawableGoal::TimeFrames(goal_time_frames.clone())),
            ),
            DataPoint::new(
                NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                Some(DrawableData::TimeFrames(time_frame_list.clone())),
                [0, 255, 0],
                Some(DrawableGoal::TimeFrames(goal_time_frames)),
            ),
        ];

        let y_axis = YAxis::TimeOfDay {
            min_seconds: 8 * 3600,
            max_seconds: 18 * 3600,
            label_format: AxisLabelFormat::TimeOfDayHhMm,
        };
        let plot_data = PlotData::new(
            "Time Frames Test Plot".to_string(),
            points,
            date_range,
            PlotType::TimeFrames,
            y_axis,
        );

        let config = RenderConfig::default();
        let result = renderer.render(&plot_data, &config);

        assert!(result.is_ok(), "Rendering should succeed");
        let png_bytes = result.unwrap();
        assert!(!png_bytes.is_empty(), "PNG data should not be empty");

        // Verify it's a valid PNG by checking magic bytes
        assert_eq!(
            &png_bytes[0..4],
            &[0x89, 0x50, 0x4E, 0x47],
            "Should be a valid PNG"
        );
    }
}
