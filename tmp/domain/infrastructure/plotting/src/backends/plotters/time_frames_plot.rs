use crate::plot::objects::mutable_image::MutableImage;
use crate::plot::objects::plot_data::PlotDataPoint;
use crate::plot::objects::y_axis_conversion::{calculate_seconds_ticks, seconds_to_time_formatter};
use crate::plot::plot_backends::plotters::common::{
    add_y_padding, color_to_plotters, get_bar, get_border, get_horizontal_line, get_streak_text,
};
use crate::plot::plot_backends::plotters::constants::{
    BACKGROUND_COLOR, GOAL_LINE_COLOR, SHADING_COLOR, Y_MARGIN_PERCENT,
};
use crate::value_objects::time_frame_list::TimeFrameList;
use anyhow::Result;
use chrono::{NaiveDate, NaiveTime, Timelike};
use plotters::prelude::*;
use transfer_objects::image::Image;

pub fn time_frames_plot(
    title: String,
    size: (usize, usize),
    data: Vec<PlotDataPoint<TimeFrameList>>,
    _y_unit: String,
    border_color: RGBColor,
    x_min: NaiveDate,
    x_max: NaiveDate,
    streak: Option<u32>,
) -> Result<Image> {
    let mut image = MutableImage::new_with_size(size.0, size.1);
    {
        let root =
            BitMapBackend::with_buffer(image.get_rgb_buffer_mut(), (size.0 as u32, size.1 as u32))
                .into_drawing_area();
        root.fill(BACKGROUND_COLOR)?;

        let (y_min, y_max) = calculate_y_axis_bounds(&data);
        let (y_axis_min, y_axis_max) = add_y_padding(
            y_min.num_seconds_from_midnight() as i64,
            y_max.num_seconds_from_midnight() as i64,
            Y_MARGIN_PERCENT,
        );

        let mut chart = ChartBuilder::on(&root)
            .x_label_area_size(25)
            .y_label_area_size(52)
            .margin_right(7)
            .caption(title, ("IPAMincho", 20).into_font().color(&WHITE))
            .build_cartesian_2d(
                (x_min..x_max).into_segmented(),
                (y_axis_min..y_axis_max)
                    .with_key_points(calculate_seconds_ticks(y_axis_min, y_axis_max, size.1)),
            )?;

        chart
            .configure_mesh()
            .x_labels((x_max - x_min).num_days() as usize + 1)
            .disable_x_mesh()
            .bold_line_style(WHITE.mix(0.2))
            .x_label_formatter(&|seg| match seg {
                SegmentValue::Exact(d) => d.format("%d.%m").to_string(),
                SegmentValue::CenterOf(d) => d.format("%d.").to_string(),
                SegmentValue::Last => "".to_string(),
            })
            .y_label_formatter(&seconds_to_time_formatter)
            .axis_style(border_color)
            .label_style(("IPAMincho", 20, &WHITE))
            .draw()?;

        // Draw bars
        chart.draw_series(
            data.iter()
                .filter(|point| point.value.is_some())
                .flat_map(|point| {
                    let value = point.value.as_ref().unwrap(); // safe due to the filter

                    value.iter().map(|time_frame| {
                        get_bar(
                            point.date,
                            time_frame.get_start().num_seconds_from_midnight() as i64,
                            time_frame.get_end().num_seconds_from_midnight() as i64,
                            2,
                            &color_to_plotters(&point.color).to_rgba(),
                        )
                    })
                }),
        )?;

        // Draw goal lines
        for data_point in data {
            for goal_time_frames in data_point.goal.iter() {
                if goal_time_frames.is_empty() {
                    continue;
                }

                let mut last_end_time = 0;

                for frame in goal_time_frames {
                    let current_start_time = frame.get_start().num_seconds_from_midnight() as i64;
                    let current_end_time = frame.get_end().num_seconds_from_midnight() as i64;

                    // Shade the area between the last end time and the current start time
                    chart.draw_series([get_bar(
                        data_point.date,
                        last_end_time,
                        current_start_time,
                        0,
                        &SHADING_COLOR,
                    )])?;

                    // Draw the goal lines
                    chart.draw_series([
                        get_horizontal_line(
                            data_point.date,
                            current_start_time,
                            GOAL_LINE_COLOR.mix(0.7),
                        ),
                        get_horizontal_line(
                            data_point.date,
                            current_end_time,
                            GOAL_LINE_COLOR.mix(0.7),
                        ),
                    ])?;

                    last_end_time = current_end_time;
                }

                // Shade the area between the last end time and the y-axis max
                chart.draw_series([get_bar(
                    data_point.date,
                    last_end_time,
                    y_axis_max,
                    0,
                    &SHADING_COLOR,
                )])?;
            }
        }

        // Draw border
        chart.draw_series([get_border(&border_color, x_min, x_max, y_axis_max)])?;

        // Draw streak
        if let Some(streak_value) = streak {
            root.draw(&get_streak_text(size.0 as u32, streak_value))?;
        }

        root.present()?;
    }
    image.into_immutable()
}

fn calculate_y_axis_bounds(data: &[PlotDataPoint<TimeFrameList>]) -> (NaiveTime, NaiveTime) {
    const DEFAULT_MIN: NaiveTime = NaiveTime::from_num_seconds_from_midnight_opt(0, 0).unwrap();
    const DEFAULT_MAX: NaiveTime = NaiveTime::from_num_seconds_from_midnight_opt(1000, 0).unwrap();

    let lower_values = data
        .iter()
        .filter_map(|dp| dp.value.as_ref().and_then(|value| value.start_time()));
    let lower_goals = data
        .iter()
        .filter_map(|dp| dp.goal.as_ref().and_then(|g| g.start_time()));
    let y_min: NaiveTime = lower_values.chain(lower_goals).min().unwrap_or(DEFAULT_MIN);

    let upper_values = data
        .iter()
        .filter_map(|dp| dp.value.as_ref().and_then(|value| value.end_time()));
    let upper_goals = data
        .iter()
        .filter_map(|dp| dp.goal.as_ref().and_then(|g| g.end_time()));

    let y_max: NaiveTime = upper_values.chain(upper_goals).max().unwrap_or(DEFAULT_MAX);

    (y_min, y_max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plot::objects::color::Color;
    use crate::value_objects::time_frame_list::{TimeFrame, TimeFrameList};
    use chrono::NaiveTime;

    fn create_plot_data_point(
        frames: Vec<(NaiveTime, NaiveTime)>,
        goal_frames: Option<Vec<(NaiveTime, NaiveTime)>>,
    ) -> PlotDataPoint<TimeFrameList> {
        let mut time_frame_list = TimeFrameList::default();
        for (start, end) in frames {
            time_frame_list
                .append_frame(TimeFrame::new(start, end).unwrap())
                .unwrap();
        }

        let goal = goal_frames.map(|frames| {
            let mut list = TimeFrameList::default();
            for (start, end) in frames {
                list.append_frame(TimeFrame::new(start, end).unwrap())
                    .unwrap();
            }
            list
        });

        PlotDataPoint::new(
            chrono::Local::now().date_naive(),
            Some(time_frame_list),
            goal,
            Color([0, 0, 0]),
        )
    }

    #[test]
    fn test_empty_data() {
        let data = Vec::new();
        let (min, max) = calculate_y_axis_bounds(&data);

        // Should return default bounds when no data
        assert_eq!(min.num_seconds_from_midnight(), 0);
        assert_eq!(max.num_seconds_from_midnight(), 1000);
    }

    #[test]
    fn test_single_timeframe() {
        let start = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let end = NaiveTime::from_hms_opt(17, 0, 0).unwrap();

        let data = vec![create_plot_data_point(vec![(start, end)], None)];

        let (min, max) = calculate_y_axis_bounds(&data);
        assert_eq!(min, start);
        assert_eq!(max, end);
    }

    #[test]
    fn test_multiple_timeframes() {
        let frames = vec![
            (
                NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
                NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
            ),
            (
                NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
            ),
        ];

        let data = vec![create_plot_data_point(frames, None)];

        let (min, max) = calculate_y_axis_bounds(&data);
        assert_eq!(min, NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        assert_eq!(max, NaiveTime::from_hms_opt(17, 0, 0).unwrap());
    }

    #[test]
    fn test_with_goals() {
        let value_frames = vec![(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        )];

        let goal_frames = vec![(
            NaiveTime::from_hms_opt(8, 0, 0).unwrap(),  // Earlier start
            NaiveTime::from_hms_opt(13, 0, 0).unwrap(), // Later end
        )];

        let data = vec![create_plot_data_point(value_frames, Some(goal_frames))];

        let (min, max) = calculate_y_axis_bounds(&data);
        assert_eq!(min, NaiveTime::from_hms_opt(8, 0, 0).unwrap()); // Should use goal's earlier start
        assert_eq!(max, NaiveTime::from_hms_opt(13, 0, 0).unwrap()); // Should use goal's later end
    }

    #[test]
    fn test_multiple_days() {
        let day1_frames = vec![(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
        )];

        let day2_frames = vec![(
            NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(16, 0, 0).unwrap(),
        )];

        let data = vec![
            create_plot_data_point(day1_frames, None),
            create_plot_data_point(day2_frames, None),
        ];

        let (min, max) = calculate_y_axis_bounds(&data);
        assert_eq!(min, NaiveTime::from_hms_opt(8, 0, 0).unwrap());
        assert_eq!(max, NaiveTime::from_hms_opt(17, 0, 0).unwrap());
    }

    #[test]
    fn test_empty_frames_with_goals() {
        let goal_frames = vec![(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
        )];

        let data = vec![create_plot_data_point(vec![], Some(goal_frames))];

        let (min, max) = calculate_y_axis_bounds(&data);
        assert_eq!(min, NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        assert_eq!(max, NaiveTime::from_hms_opt(17, 0, 0).unwrap());
    }

    #[test]
    fn test_empty_goals_with_frames() {
        let frames = vec![(
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
            NaiveTime::from_hms_opt(17, 0, 0).unwrap(),
        )];

        let data = vec![create_plot_data_point(frames, Some(vec![]))];

        let (min, max) = calculate_y_axis_bounds(&data);
        assert_eq!(min, NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        assert_eq!(max, NaiveTime::from_hms_opt(17, 0, 0).unwrap());
    }
}
