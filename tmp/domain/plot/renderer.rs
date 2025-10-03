// domain/src/plot/renderer.rs
// NEW FILE

/// Trait defining the contract between domain and infrastructure for plot rendering.
///
/// The domain prepares WHAT to render (data, configuration), and the infrastructure
/// decides HOW to render it (which library, output format, etc.).
///
/// This separation allows:
/// - Testing domain logic without rendering
/// - Multiple rendering implementations (PNG, SVG, HTML Canvas)
/// - Keeping domain free of infrastructure dependencies
pub trait PlotRenderer: Send + Sync {
    /// Renders plot data into an image.
    ///
    /// # Arguments
    /// * `data` - The plot data prepared by domain logic
    /// * `config` - Rendering configuration (size, colors, etc.)
    ///
    /// # Returns
    /// Raw image bytes (format depends on implementation - PNG, SVG, etc.)
    fn render(&self, data: &PlotData, config: &RenderConfig) -> Result<Vec<u8>>;

    /// Returns the MIME type of the rendered output (e.g., "image/png", "image/svg+xml")
    fn output_format(&self) -> &str;
}
