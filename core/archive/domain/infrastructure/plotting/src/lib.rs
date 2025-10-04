// infrastructure/plotting/src/lib.rs

mod plotters_backend;
pub use plotters_backend::PlottersRenderer;

// Re-export the trait for convenience
pub use lifestats_domain::plot::renderer::PlotRenderer;
