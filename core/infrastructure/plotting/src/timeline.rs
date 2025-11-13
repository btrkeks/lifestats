use anyhow::Result;
use chrono::{NaiveDate, Timelike};
use image::{ImageBuffer, Rgb};
use lifestats_core::shared::time_frame_list::TimeFrame;
use plot_domain::plot::renderer::RenderConfig;
use plotters::prelude::*;
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};

use crate::common::{calculate_seconds_ticks, seconds_to_time_formatter, BACKGROUND_COLOR};

/// Represents a timeline event for plotting purposes
/// This is a simple wrapper to avoid circular dependencies with the application layer
#[derive(Debug, Clone)]
pub struct TimelineEventData {
    pub timeframe: TimeFrame,
    pub label: String,
}

/// Generates a deterministic color from a label string
/// Same label will always produce the same color
fn color_from_label(label: &str) -> RGBColor {
    let mut hasher = DefaultHasher::new();
    label.hash(&mut hasher);
    let hash = hasher.finish();

    // Generate vibrant colors by ensuring at least one channel is bright
    // and avoiding too-dark colors
    let r = ((hash & 0xFF) as u8).max(60);
    let g = (((hash >> 8) & 0xFF) as u8).max(60);
    let b = (((hash >> 16) & 0xFF) as u8).max(60);

    // Ensure at least one channel is quite bright for visibility
    let max_channel = r.max(g).max(b);
    if max_channel < 180 {
        // Boost the brightest channel
        let boost = 180 - max_channel;
        let (r, g, b) = if r == max_channel {
            (r + boost, g, b)
        } else if g == max_channel {
            (r, g + boost, b)
        } else {
            (r, g, b + boost)
        };
        RGBColor(r, g, b)
    } else {
        RGBColor(r, g, b)
    }
}

/// Builds legend entries with unique labels and their colors
/// Returns sorted list for consistent display
fn build_legend_entries(events: &[TimelineEventData]) -> Vec<(String, RGBColor)> {
    let mut label_map: HashMap<String, RGBColor> = HashMap::new();

    for event in events {
        if !label_map.contains_key(&event.label) {
            let color = color_from_label(&event.label);
            label_map.insert(event.label.clone(), color);
        }
    }

    let mut entries: Vec<(String, RGBColor)> = label_map.into_iter().collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
}

/// Plots timeline events for a single day
///
/// This function renders a timeline visualization showing time frames vertically,
/// with the y-axis representing time of day (00:00-24:00) and colored bars showing
/// different activities in a single column layout with a legend.
///
/// # Arguments
/// * `events` - List of timeline events to plot
/// * `date` - The date being visualized
/// * `config` - Rendering configuration (width, height)
///
/// # Returns
/// * `Result<Vec<u8>>` - PNG image bytes on success
pub fn plot_timeline_events(
    events: Vec<TimelineEventData>,
    date: NaiveDate,
    config: RenderConfig,
) -> Result<Vec<u8>> {
    // Create image buffer
    let mut buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(config.width, config.height);
    let root =
        BitMapBackend::with_buffer(&mut buffer, (config.width, config.height)).into_drawing_area();

    root.fill(&BACKGROUND_COLOR)?;

    // Y-axis: full day in seconds (00:00 to 24:00)
    let y_min = 0i64;
    let y_max = 86400i64; // 24 hours * 3600 seconds

    // Build chart with single column (0.0 to 1.0)
    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(0) // No X labels needed for single column
        .y_label_area_size(52)
        .margin_right(120) // Extra space for legend
        .margin_top(30)
        .caption(
            &format!("Timeline - {}", date.format("%Y-%m-%d")),
            ("IPAMincho", 20).into_font().color(&WHITE),
        )
        .build_cartesian_2d(
            0.0..1.0,
            (y_min..y_max).with_key_points(calculate_seconds_ticks(
                y_min,
                y_max,
                config.height as usize,
            )),
        )?;

    chart
        .configure_mesh()
        .disable_x_mesh()
        .bold_line_style(WHITE.mix(0.2))
        .y_label_formatter(&seconds_to_time_formatter)
        .axis_style(WHITE)
        .label_style(("IPAMincho", 15, &WHITE))
        .draw()?;

    // Draw event bars
    for event in &events {
        let start_seconds = event.timeframe.get_start().num_seconds_from_midnight() as i64;
        let end_seconds = event.timeframe.get_end().num_seconds_from_midnight() as i64;
        let color = color_from_label(&event.label);

        // Draw bar in center of single column (0.1 to 0.9 for some margin)
        chart.draw_series(std::iter::once(Rectangle::new(
            [(0.1, start_seconds), (0.9, end_seconds)],
            color.filled(),
        )))?;
    }

    // Draw legend
    let legend_entries = build_legend_entries(&events);
    if !legend_entries.is_empty() {
        let legend_x_start = 0.95;
        let legend_y_start = y_min + 1000; // Start a bit below the top
        let legend_box_size = 1200i64; // Size of color box in seconds
        let legend_spacing = 1800i64; // Spacing between entries in seconds

        for (idx, (_label, color)) in legend_entries.iter().enumerate() {
            let y_pos = legend_y_start + (idx as i64 * legend_spacing);

            // Draw color box
            chart.draw_series(std::iter::once(Rectangle::new(
                [
                    (legend_x_start, y_pos),
                    (legend_x_start + 0.03, y_pos + legend_box_size),
                ],
                color.filled(),
            )))?;

            // Draw label text (positioned to the right of the box, but this requires
            // drawing directly on root since chart coordinates don't work well for text)
        }

        // Draw legend text directly on root area for better positioning
        let legend_pixel_x = (config.width as f64 * 0.82) as i32;
        let legend_pixel_y_start = 80i32;
        let legend_pixel_spacing = 25i32;

        for (idx, (label, color)) in legend_entries.iter().enumerate() {
            let y = legend_pixel_y_start + (idx as i32 * legend_pixel_spacing);

            // Draw color box
            root.draw(&Rectangle::new(
                [(legend_pixel_x, y), (legend_pixel_x + 15, y + 15)],
                color.filled(),
            ))?;

            // Draw label text
            root.draw(&Text::new(
                label.as_str(),
                (legend_pixel_x + 20, y),
                ("IPAMincho", 14).into_font().color(&WHITE),
            ))?;
        }
    }

    // Draw border
    chart.draw_series(std::iter::once(Rectangle::new(
        [(0.0, y_min), (1.0, y_max)],
        WHITE.stroke_width(1),
    )))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveTime;

    #[test]
    fn test_color_from_label_deterministic() {
        // Same label should always produce same color
        let color1 = color_from_label("Study Time");
        let color2 = color_from_label("Study Time");
        assert_eq!(color1, color2);

        // Different labels should (usually) produce different colors
        let color3 = color_from_label("Exercise");
        assert_ne!(color1, color3);
    }

    #[test]
    fn test_color_from_label_vibrant() {
        // Test that colors are not too dark
        let color = color_from_label("Test");
        let (r, g, b) = (color.0, color.1, color.2);

        // At least one channel should be reasonably bright
        let max_channel = r.max(g).max(b);
        assert!(
            max_channel >= 180,
            "Color should be vibrant, got RGB({}, {}, {})",
            r,
            g,
            b
        );

        // No channel should be too dark
        assert!(r >= 60, "Red channel too dark");
        assert!(g >= 60, "Green channel too dark");
        assert!(b >= 60, "Blue channel too dark");
    }

    #[test]
    fn test_build_legend_entries_unique_sorted() {
        let events = vec![
            TimelineEventData {
                timeframe: TimeFrame::new(
                    NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                    NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                )
                .unwrap(),
                label: "Zebra".to_string(),
            },
            TimelineEventData {
                timeframe: TimeFrame::new(
                    NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                    NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
                )
                .unwrap(),
                label: "Apple".to_string(),
            },
            TimelineEventData {
                timeframe: TimeFrame::new(
                    NaiveTime::from_hms_opt(11, 0, 0).unwrap(),
                    NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
                )
                .unwrap(),
                label: "Zebra".to_string(), // Duplicate
            },
        ];

        let legend = build_legend_entries(&events);

        // Should have 2 unique entries
        assert_eq!(legend.len(), 2);

        // Should be sorted alphabetically
        assert_eq!(legend[0].0, "Apple");
        assert_eq!(legend[1].0, "Zebra");

        // Same label should have same color in legend
        let zebra_color = legend.iter().find(|(l, _)| l == "Zebra").unwrap().1;
        assert_eq!(zebra_color, color_from_label("Zebra"));
    }

    #[test]
    fn test_plot_timeline_with_single_event() {
        let events = vec![TimelineEventData {
            timeframe: TimeFrame::new(
                NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            )
            .unwrap(),
            label: "Study Time".to_string(),
        }];

        let date = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();
        let config = RenderConfig::default();

        let result = plot_timeline_events(events, date, config);

        assert!(result.is_ok(), "Should successfully generate plot");
        let png_bytes = result.unwrap();
        assert!(!png_bytes.is_empty(), "PNG data should not be empty");

        // Verify PNG magic bytes
        assert_eq!(
            &png_bytes[0..4],
            &[0x89, 0x50, 0x4E, 0x47],
            "Should be a valid PNG"
        );
    }

    #[test]
    fn test_plot_timeline_with_multiple_events() {
        let events = vec![
            TimelineEventData {
                timeframe: TimeFrame::new(
                    NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                    NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
                )
                .unwrap(),
                label: "Exercise".to_string(),
            },
            TimelineEventData {
                timeframe: TimeFrame::new(
                    NaiveTime::from_hms_opt(10, 0, 0).unwrap(),
                    NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
                )
                .unwrap(),
                label: "Study Time".to_string(),
            },
            TimelineEventData {
                timeframe: TimeFrame::new(
                    NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                    NaiveTime::from_hms_opt(15, 0, 0).unwrap(),
                )
                .unwrap(),
                label: "Study Time".to_string(), // Same label
            },
        ];

        let date = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();
        let config = RenderConfig::default();

        let result = plot_timeline_events(events, date, config);

        assert!(result.is_ok(), "Should successfully generate plot");
        let png_bytes = result.unwrap();
        assert!(!png_bytes.is_empty(), "PNG data should not be empty");
        assert_eq!(&png_bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_plot_timeline_empty_events() {
        let events = vec![];
        let date = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();
        let config = RenderConfig::default();

        let result = plot_timeline_events(events, date, config);

        // Should successfully create an empty timeline
        assert!(result.is_ok(), "Empty timeline should render successfully");
        let png_bytes = result.unwrap();
        assert!(!png_bytes.is_empty());
        assert_eq!(&png_bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_plot_timeline_full_day_coverage() {
        let events = vec![
            TimelineEventData {
                timeframe: TimeFrame::new(
                    NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                    NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
                )
                .unwrap(),
                label: "Sleep".to_string(),
            },
            TimelineEventData {
                timeframe: TimeFrame::new(
                    NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
                    NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
                )
                .unwrap(),
                label: "Work".to_string(),
            },
            TimelineEventData {
                timeframe: TimeFrame::new(
                    NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
                    NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
                )
                .unwrap(),
                label: "Leisure".to_string(),
            },
        ];

        let date = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();
        let config = RenderConfig::default();

        let result = plot_timeline_events(events, date, config);

        assert!(
            result.is_ok(),
            "Full day timeline should render successfully"
        );
        let png_bytes = result.unwrap();
        assert!(!png_bytes.is_empty());
        assert_eq!(&png_bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_plot_timeline_with_custom_dimensions() {
        let events = vec![TimelineEventData {
            timeframe: TimeFrame::new(
                NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
                NaiveTime::from_hms_opt(13, 0, 0).unwrap(),
            )
            .unwrap(),
            label: "Lunch".to_string(),
        }];

        let date = NaiveDate::from_ymd_opt(2025, 11, 1).unwrap();
        let config = RenderConfig::new(1920, 1080);

        let result = plot_timeline_events(events, date, config);

        assert!(result.is_ok(), "Custom dimensions should work");
        let png_bytes = result.unwrap();
        assert!(!png_bytes.is_empty());
        assert_eq!(&png_bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }
}
