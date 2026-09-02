pub mod daily_achievement_evaluator;
pub mod ewma_calculator;
pub mod goal_performance_analyzer;
pub mod goal_resolver;
pub mod plot_data_builder;
pub mod statistic_score_calculator;
pub mod value_averaging;

pub use daily_achievement_evaluator::{
    DailyAchievementEvaluator, DayEval, DefaultDailyAchievementEvaluator,
};
pub use ewma_calculator::Ewma;
pub use goal_performance_analyzer::GoalPerformanceAnalyzer;
pub use goal_resolver::{any_goal_to_json, GoalResolver};
pub use statistic_score_calculator::compute_mean_percentage;

#[cfg(test)]
pub use daily_achievement_evaluator::MockDailyAchievementEvaluator;
