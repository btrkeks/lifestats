use crate::value_objects::date_range::DateRange;
use crate::value_objects::dated_value::DatedValue;
use crate::value_objects::percentage::Percentage;
use crate::plot::objects::color::Color;
use crate::plot::objects::plot_data::PlotDataPoint;
use crate::plot::util::percent_to_color::percent_to_color;
use crate::statistic::traits::integer_statistic::IntegerStatistic;
use std::sync::Arc;

pub struct IntegerPlotData {
    title: String,
    date_range: DateRange,
    data_points: Vec<PlotDataPoint<i64>>,
    streak: u32,
    border_color: Color,
}

impl IntegerPlotData {
    pub fn from_integer_statistic<G, L>(
        title: String,
        date_range: DateRange,
        stat: Arc<dyn IntegerStatistic<GoalType = G, LogValueType = L>>,
    ) -> anyhow::Result<Self> {
        let values = stat.get_integer_data_closed_range(date_range.start(), date_range.end());
        let goals = stat.get_integer_goals_closed_range(date_range.start(), date_range.end());
        let scores = stat.get_scores_closed_range(date_range.start(), date_range.end())?;
        let border_color = Self::get_border_color(&scores);

        let data_points = values
            .into_iter()
            .zip(goals)
            .zip(scores)
            .map(|((value, goal), score)| {
                debug_assert_eq!(value.date, goal.date);
                debug_assert_eq!(value.date, score.date);

                PlotDataPoint {
                    date: value.date,
                    value: value.value,
                    goal: goal.value,
                    color: percent_to_color(score.value),
                }
            })
            .collect();


        let streak = stat.get_streak()?;

        Ok(Self {
            title,
            date_range,
            data_points,
            streak,
            border_color,
        })
    }

    /// Calculates the border color depending on the scores
    fn get_border_color(scores: &[DatedValue<Percentage>]) -> Color {
        // TODO: This probably doesn't belong here
        // Calculate the mean score
        let mean_score = if scores.is_empty() {
            None
        }
        else {
            let sum = scores.iter().map(|dv| dv.value.as_num()).sum();
            Some(sum / (scores.len() as i64))
        };

        percent_to_color(mean_score)
    }
}
