// domain/src/plot/specification.rs
// NEW FILE

/// Configuration that affects HOW a plot should be rendered.
///
/// While PlotData represents WHAT to render (domain concern),
/// RenderConfig represents HOW to render it (presentation concern).
///
/// The domain can provide defaults, but the infrastructure
/// (or caller) can override these for different output contexts
/// (desktop wallpaper vs web dashboard).
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Output dimensions in pixels
    pub width: u32,
    pub height: u32,

    /// Whether to use dark or light theme
    pub theme: Theme,

    /// Optional: specific color overrides
    pub colors: Option<ColorScheme>,
}

#[derive(Debug, Clone, Copy)]
pub enum Theme {
    Light,
    Dark,
}

#[derive(Debug, Clone)]
pub struct ColorScheme {
    pub background: Color,
    pub foreground: Color,
    pub accent: Color,
    pub goal_line: Color,
}

// Note: Keep Color as a simple value object (RGB)
// Don't import image crate types here!
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            theme: Theme::Dark,
            colors: None, // Use theme defaults
        }
    }
}
