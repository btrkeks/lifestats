// application/src/use_cases/plot_generation.rs
// NEW FILE

use anyhow::Result;
use lifestats_domain::{
    plot::{PlotData, PlotRenderer, RenderConfig},
    statistic::StatisticId,
    value_objects::DateRange,
};

/// Application service for generating plots.
///
/// This orchestrates the workflow:
/// 1. Fetch data from repositories (infrastructure)
/// 2. Prepare plot data using domain logic (domain)
/// 3. Render using injected renderer (infrastructure)
///
/// Note: This doesn't know WHICH renderer is used - that's
/// decided by whoever constructs AppContext.
pub fn generate_plot(
    ctx: &AppContext,
    statistic_id: StatisticId,
    date_range: DateRange,
    config: RenderConfig,
) -> Result<Vec<u8>> {
    // Step 1: Get data from repositories (infrastructure)
    let logs = ctx.log_repository.get_logs(statistic_id, date_range)?;
    let goal = ctx.goal_repository.get_goal(statistic_id).ok();

    // Step 2: Domain prepares the plot data (domain logic)
    // This is where business rules are applied:
    // - Which data points to include
    // - How to calculate goal percentages
    // - Which visualization type is appropriate
    let plot_data = PlotData::from_statistic_logs(logs, goal, date_range)?;

    // Step 3: Infrastructure renders it (implementation detail)
    // The renderer was injected into AppContext, so we don't
    // care if it's plotters, SVG, Canvas, or a mock
    ctx.plot_renderer.render(&plot_data, &config)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockRenderer;

    impl PlotRenderer for MockRenderer {
        fn render(&self, data: &PlotData, config: &RenderConfig) -> Result<Vec<u8>> {
            // Return fake image bytes for testing
            Ok(vec![1, 2, 3, 4])
        }

        fn output_format(&self) -> &str {
            "test/mock"
        }
    }

    #[test]
    fn test_plot_generation_workflow() {
        let ctx = MockAppContextBuilder::new()
            .with_mock_repositories()
            .with_renderer(Box::new(MockRenderer))
            .build();

        let result = generate_plot(
            &ctx,
            StatisticId::from("sleep"),
            DateRange::last_7_days(),
            RenderConfig::default(),
        );

        assert!(result.is_ok());
    }
}
