use crate::plot::data::PlotData;
use anyhow::Result;

/// Controls whether each data point is annotated with its exact value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValueLabels {
    /// Show labels when there is enough horizontal room per day column.
    #[default]
    Auto,
    /// Always draw labels, even if they may crowd each other.
    Show,
    /// Never draw labels.
    Hide,
}

/// Configuration for rendering a plot
/// Contains presentation details like dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderConfig {
    pub width: u32,
    pub height: u32,
    pub value_labels: ValueLabels,
}

impl RenderConfig {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            value_labels: ValueLabels::Auto,
        }
    }

    pub fn with_value_labels(mut self, value_labels: ValueLabels) -> Self {
        self.value_labels = value_labels;
        self
    }
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self::new(800, 600)
    }
}

/// Port (trait) for rendering plots
/// Any concrete rendering implementation must accept domain PlotData and return image bytes
/// This keeps the domain independent of specific rendering libraries
#[cfg_attr(feature = "testing", mockall::automock)]
pub trait PlotRenderer: Send + Sync {
    /// Renders plot data into raw image bytes
    fn render(&self, data: &PlotData, config: &RenderConfig) -> Result<Vec<u8>>;
}
