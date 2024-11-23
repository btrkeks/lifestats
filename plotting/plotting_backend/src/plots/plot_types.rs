use std::path::Path;
use crate::plotters_backend;
use anyhow::{anyhow, Result};
use backend::statistics::objects::{StatisticId, TimeFrame};
use chrono::NaiveDate;
use enum_dispatch::enum_dispatch;
use log::error;
use plotters::prelude::{RGBColor, WHITE};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use backend::statistics::statistic_definition_storage::{get_all_statistics, get_statistic_by_id};
use backend::statistics::type_definitions::{AnyStatistic, Statistic, StatisticTypeSpecific};
use backend::statistics::type_definitions::point_in_time::PointInTimeStatistic;
use backend::statistics::type_definitions::time_spans::TimeSpansStatistic;
use crate::plots::plot_data::{BarplotData, BarplotDataPoint, LabeledTime, LabeledTimeFrame, TimeFramesPlotData, TimeFramesPlotPoint, TimePointPlotData, TimePointPlotPoint, TimelineData, YValueFormatting};
use crate::util::color_consts;

pub fn perc_to_color(perc: Option<f64>) -> RGBColor {
    match perc {
        None => WHITE,
        Some(perc) => {
            if perc >= 1.5 {
                color_consts::GOLD
            } else if perc >= 1.0 {
                color_consts::GREEN
            } else if perc >= 0.9 {
                color_consts::ORANGE
            } else {
                color_consts::RED
            }
        }
    }
}

#[enum_dispatch]
pub trait Plot {
    fn get_id(&self) -> u32;
    fn get_title(&self) -> &str;
    fn plot(&self, save_dir: &Path, conn: &Connection, start_date: NaiveDate, end_date: NaiveDate) -> Result<()>;
    // fn get_data(&self, conn: &Connection, start: NaiveDate, end: NaiveDate) -> Result<Box<dyn PlotData>>;
    fn contains_statistic_with_id(&self, id: StatisticId) -> bool;
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeLinePlot {
    id: u32,
    title: String,
    statistic_ids: Vec<StatisticId>,
    time_frame: TimeFrame,
}

impl Plot for TimeLinePlot {
    fn get_id(&self) -> u32 {
        self.id
    }

    fn get_title(&self) -> &str {
        &self.title
    }

    fn plot(&self, save_dir: &Path, conn: &Connection, _start_date: NaiveDate, end_date: NaiveDate) -> Result<()> {
        let filename = format!("{}.png", self.get_id());
        let plot_path = save_dir.join(filename);
        let data = self.get_data(&conn, end_date)?;

        plotters_backend::plot_timeline(&plot_path, &self.get_title(), data)
    }

    fn contains_statistic_with_id(&self, id: StatisticId) -> bool {
        self.statistic_ids.contains(&id)
    }
}

impl TimeLinePlot {
    pub fn get_data(&self, conn: &Connection, date: NaiveDate) -> Result<TimelineData> {
        let statistics = get_all_statistics()?;

        let (time_frames, time_points) = self.statistic_ids.iter().try_fold(
            (Vec::new(), Vec::new()),
            |(mut frames, mut points), id| -> Result<(Vec<LabeledTimeFrame>, Vec<LabeledTime>)> {
                let statistic = self.find_statistic(&statistics, *id)?;
                match statistic {
                    AnyStatistic::TimeSpans(s) => {
                        frames.extend(Self::get_labeled_time_frames(conn, s, date)?);
                    }
                    AnyStatistic::PointInTime(s) => {
                        if let Some(point) = Self::get_labeled_time_point(conn, s, date)? {
                            points.push(point);
                        }
                    }
                    _ => error!("Statistic with id {} is not a TimeSpansStatistic!", id),
                }
                Ok((frames, points))
            },
        )?;

        Ok(TimelineData {
            time_frames,
            time_points,
        })
    }

    // TODO: This doesn't belong here
    fn find_statistic<'a>(
        &self,
        statistics: &'a [AnyStatistic],
        id: StatisticId,
    ) -> Result<&'a AnyStatistic> {
        statistics.iter().find(|s| s.get_id() == id).ok_or_else(|| {
            anyhow!(
                "Couldn't find statistic with id {} for timeline with id {}",
                id,
                self.id
            )
        })
    }

    fn get_labeled_time_frames(
        conn: &Connection,
        statistic: &TimeSpansStatistic,
        date: NaiveDate,
    ) -> Result<impl IntoIterator<Item=LabeledTimeFrame>> {
        let row = statistic.get_row_for_date(conn, date)?;
        let name = statistic.get_name().to_string();
        let id = statistic.get_id();
        Ok(row.value.into_iter().flatten().map(move |time_frame| {
            LabeledTimeFrame::from_time_frame(id, name.clone(), time_frame, None)
        }))
    }

    fn get_labeled_time_point(
        conn: &Connection,
        statistic: &PointInTimeStatistic,
        date: NaiveDate,
    ) -> Result<Option<LabeledTime>> {
        let row = statistic.get_row_for_date(conn, date)?;
        Ok(row.value.map(|value| LabeledTime {
            label: statistic.get_name().to_string(),
            time: value,
        }))
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimeSeriesPlot {
    id: u32,
    title: String,
    statistic_id: StatisticId,
}

impl Plot for TimeSeriesPlot {
    fn get_id(&self) -> u32 {
        self.id
    }

    fn get_title(&self) -> &str {
        &self.title
    }

    fn plot(&self, save_dir: &Path, conn: &Connection, start_date: NaiveDate, end_date: NaiveDate) -> Result<()> {
        let filename = format!("{}.png", self.get_id());
        let plot_path = save_dir.join(filename);
        let data = self.get_data_for_range(&conn, start_date, end_date)?;

        plotters_backend::plot_time_frames(&plot_path, &self.get_title(), start_date, end_date, &data)
    }

    fn contains_statistic_with_id(&self, id: StatisticId) -> bool {
        self.statistic_id == id
    }
}

impl TimeSeriesPlot {
    pub fn get_data_for_range(
        &self,
        conn: &Connection,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<TimeFramesPlotData> {
        let statistic = get_statistic_by_id(self.statistic_id)?.ok_or_else(|| {
            anyhow!(
                "Couldn't find statistic with id {} for TimeSeriesPlot with id {}",
                self.statistic_id,
                self.id
            )
        })?;

        match statistic {
            AnyStatistic::TimeSpans(s) => {
                let rows = s.get_rows_for_range(conn, start, end)?;
                let total_percent = s.calculate_mean_score(&rows)
                    .map(|perc| perc + 0.05); // 勘弁して..

                Ok(TimeFramesPlotData {
                    border_color: perc_to_color(total_percent),
                    data_points: rows
                        .iter()
                        .map(|row| {
                            let percent = s.calculate_percent_in_relation_to_goal(row);
                            TimeFramesPlotPoint {
                                date: row.date,
                                goal_time_frames: row.goal.clone().unwrap_or_default(),
                                time_frames: row.value.clone().unwrap_or_default(),
                                color: perc_to_color(percent),
                            }
                        })
                        .collect(),
                })
            }
            // AnyStatistic::TimeSpan(s) => {
            //     let rows = s.get_rows_for_range(conn, start, end)?;
            //     let total_percent = s.calculate_mean_score(&rows)
            //         .map(|perc| perc + 0.05); // 勘弁して..
            //
            //     Ok(TimeFramesPlotData {
            //         border_color: perc_to_color(total_percent),
            //         data_points: rows
            //             .iter()
            //             .map(|row| {
            //                 let percent = s.calculate_percent_in_relation_to_goal(row);
            //
            //                 TimeFramesPlotPoint {
            //                     date: row.date,
            //                     goal_time_frames: row
            //                         .goal
            //                         .map(|x| TimeFrameList::new_from_vec(vec![x]))
            //                         .unwrap_or_default(),
            //                     time_frames: row
            //                         .value
            //                         .map(|x| TimeFrameList::new_from_vec(vec![x]))
            //                         .unwrap_or_default(),
            //                     color: perc_to_color(percent),
            //                 }
            //             })
            //             .collect(),
            //     })
            // }
            _ => Err(anyhow!(
                "Statistic with id {} cannot be plotted as a TimeSeriesPlot",
                self.statistic_id
            ))?,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BarPlot {
    id: u32,
    title: String,
    statistic_id: StatisticId,
}

impl Plot for BarPlot {
    fn get_id(&self) -> u32 {
        self.id
    }

    fn get_title(&self) -> &str {
        &self.title
    }

    fn plot(&self, save_dir: &Path, conn: &Connection, start_date: NaiveDate, end_date: NaiveDate) -> Result<()> {
        let filename = format!("{}.png", self.get_id());
        let plot_path = save_dir.join(filename);
        let data = self.get_data_for_range(&conn, start_date, end_date)?;

        plotters_backend::plot_barplot(&plot_path, self.get_title(), start_date, end_date, &data)
    }

    fn contains_statistic_with_id(&self, id: StatisticId) -> bool {
        self.statistic_id == id
    }
}

impl BarPlot {
    pub fn get_data_for_range(
        &self,
        conn: &Connection,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<BarplotData> {
        let statistic = get_statistic_by_id(self.statistic_id)?.ok_or_else(|| {
            anyhow!(
                "Couldn't find statistic with id {} for BarPlot of id {}",
                self.statistic_id,
                self.id
            )
        })?;

        match statistic {
            AnyStatistic::IntegerAmount(s) => {
                let rows = s.get_rows_for_range(conn, start, end)?;
                let total_percent = s.calculate_mean_score(&rows)
                    .map(|perc| perc + 0.05); // 勘弁して..

                Ok(BarplotData {
                    y_value_formatting: YValueFormatting::None,
                    border_color: perc_to_color(total_percent),
                    data_points: rows
                        .iter()
                        .map(|row| {
                            let percent = s.calculate_percent_in_relation_to_goal(row);

                            BarplotDataPoint {
                                date: row.date,
                                value: row.value.unwrap_or_default(),
                                goal: row.goal,
                                color: perc_to_color(percent),
                            }
                        })
                        .collect(),
                })
            }
            AnyStatistic::TimeAmount(s) => {
                let rows = s.get_rows_for_range(conn, start, end)?;
                let total_percent = s.calculate_mean_score(&rows)
                    .map(|perc| perc + 0.05); // 勘弁して..

                Ok(BarplotData {
                    y_value_formatting: YValueFormatting::SecondsToTime,
                    border_color: perc_to_color(total_percent),
                    data_points: rows
                        .iter()
                        .map(|row| BarplotDataPoint {
                            date: row.date,
                            value: row.value.unwrap_or_default().num_seconds(),
                            goal: row.goal.map(|g| g.num_seconds()),
                            color: perc_to_color(
                                s.get_percent_towards_goal(conn, row.date).unwrap(),
                            ),
                        })
                        .collect(),
                })
            }
            AnyStatistic::TimeSpans(s) => {
                let rows = s.get_rows_for_range(conn, start, end)?;
                let total_percent = s.calculate_mean_score(&rows)
                    .map(|perc| perc + 0.05); // 勘弁して..

                Ok(BarplotData {
                    y_value_formatting: YValueFormatting::SecondsToTime,
                    border_color: perc_to_color(total_percent),
                    data_points: rows
                        .iter()
                        .map(|row| BarplotDataPoint {
                            date: row.date,
                            value: row
                                .value
                                .as_ref()
                                .map(|list| list.get_total_duration().num_seconds())
                                .unwrap_or_default(),
                            goal: row
                                .goal
                                .as_ref()
                                .map(|g| g.get_total_duration().num_seconds()),
                            color: perc_to_color(
                                s.get_percent_towards_goal(conn, row.date).unwrap(),
                            ),
                        })
                        .collect(),
                })
            }
            _ => Err(anyhow!(
                "Statistic with id {} is not an IntegerAmount",
                self.statistic_id
            ))?,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TimePointPlot {
    id: u32,
    title: String,
    statistic_id: StatisticId,
}

impl Plot for TimePointPlot {
    fn get_id(&self) -> u32 {
        self.id
    }

    fn get_title(&self) -> &str {
        &self.title
    }

    fn plot(&self, save_dir: &Path, conn: &Connection, start_date: NaiveDate, end_date: NaiveDate) -> Result<()> {
        let filename = format!("{}.png", self.get_id());
        let plot_path = save_dir.join(filename);
        let data = self.get_data_for_range(conn, start_date, end_date)?;

        plotters_backend::plot_time_points(&plot_path, self.get_title(), start_date, end_date, data)
    }

    fn contains_statistic_with_id(&self, id: StatisticId) -> bool {
        self.statistic_id == id
    }
}

impl TimePointPlot {
    pub fn get_data_for_range(
        &self,
        conn: &Connection,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<TimePointPlotData> {
        let statistic = get_statistic_by_id(self.statistic_id)?.ok_or_else(|| {
            anyhow!(
                "Couldn't find statistic with id {} for TimePointPlot of id {}",
                self.statistic_id,
                self.id
            )
        })?;

        if let AnyStatistic::PointInTime(s) = statistic {
            let rows = s.get_rows_for_range(conn, start, end)?;
            let total_percent = s.calculate_mean_score(&rows)
                .map(|perc| perc + 0.05); // 勘弁して..

            Ok(TimePointPlotData {
                border_color: perc_to_color(total_percent),
                data_points: rows
                    .iter()
                    .filter_map(|row| {
                        row.value.map(|time| {
                            let percent = s.calculate_percent_in_relation_to_goal(row);
                            TimePointPlotPoint {
                                date: row.date,
                                time,
                                goal: row.goal,
                                color: perc_to_color(percent),
                            }
                        })
                    })
                    .collect(),
            })
        } else {
            Err(anyhow!(
                "Statistic with id {} is not a PointInTimeStatistic",
                self.statistic_id
            ))?
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
#[enum_dispatch(Plot)]
pub enum AnyPlot {
    TimeSeries(TimeSeriesPlot),
    TimeLine(TimeLinePlot),
    Bar(BarPlot),
    TimePoint(TimePointPlot),
}
