use crate::plot::data::PlotData;
use anyhow::Result;

/// Configuration for rendering a plot
/// Contains presentation details like dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderConfig {
    pub width: u32,
    pub height: u32,
}

impl RenderConfig {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
        }
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
