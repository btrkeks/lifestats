pub mod daily_achievement_evaluator;
pub mod plot_data_builder;
pub mod statistic_score_calculator;

pub use daily_achievement_evaluator::{
    DailyAchievementEvaluator, DayEval, DefaultDailyAchievementEvaluator,
};
pub use statistic_score_calculator::compute_mean_percentage;

#[cfg(test)]
pub use daily_achievement_evaluator::MockDailyAchievementEvaluator;
