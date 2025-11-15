/// Exponentially Weighted Moving Average calculator
///
/// EWMA gives more weight to recent values while still considering historical data.
/// The span parameter controls how much weight is given to recent values:
/// - Smaller span = more responsive to recent changes
/// - Larger span = smoother, more stable average
#[derive(Debug, Clone)]
pub struct Ewma {
    /// Smoothing factor (alpha = 2.0 / (span + 1.0))
    alpha: f64,
    /// Current EWMA value
    value: f64,
}

impl Ewma {
    /// Create a new EWMA calculator with the specified span and initial value
    ///
    /// # Arguments
    /// * `span` - Number of periods for the moving average (must be > 0)
    /// * `initial_value` - Starting value for the EWMA
    ///
    /// # Panics
    /// Panics if span is 0
    pub fn from_span(span: u32, initial_value: f64) -> Self {
        assert!(span > 0, "EWMA span must be greater than 0");
        let alpha = 2.0 / (f64::from(span) + 1.0);
        Self {
            alpha,
            value: initial_value,
        }
    }

    /// Add a new value to the EWMA calculation
    ///
    /// Updates the internal EWMA value using the formula:
    /// `ewma_new = alpha * value + (1 - alpha) * ewma_old`
    pub fn add(&mut self, value: f64) {
        self.value = self.alpha * value + (1.0 - self.alpha) * self.value;
    }

    /// Get the current EWMA value
    pub fn value(&self) -> f64 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "EWMA span must be greater than 0")]
    fn test_ewma_zero_span_panics() {
        Ewma::from_span(0, 100.0);
    }

    #[test]
    fn test_ewma_initial_value() {
        let ewma = Ewma::from_span(3, 100.0);
        assert_eq!(ewma.value(), 100.0);
    }

    #[test]
    fn test_ewma_single_update() {
        let mut ewma = Ewma::from_span(3, 100.0);
        ewma.add(80.0);

        // With span=3, alpha = 2/(3+1) = 0.5
        // New value = 0.5 * 80.0 + 0.5 * 100.0 = 90.0
        assert!((ewma.value() - 90.0).abs() < 0.001);
    }

    #[test]
    fn test_ewma_multiple_updates() {
        let mut ewma = Ewma::from_span(3, 100.0);

        // Sequence: 100 -> 80 -> 90 -> 85
        ewma.add(80.0); // 0.5*80 + 0.5*100 = 90
        ewma.add(90.0); // 0.5*90 + 0.5*90 = 90
        ewma.add(85.0); // 0.5*85 + 0.5*90 = 87.5

        assert!((ewma.value() - 87.5).abs() < 0.001);
    }

    #[test]
    fn test_ewma_longer_span_smoother() {
        let mut fast = Ewma::from_span(3, 100.0);
        let mut slow = Ewma::from_span(7, 100.0);

        // Add a sudden drop
        fast.add(50.0);
        slow.add(50.0);

        // Fast EWMA should react more strongly
        // Fast: alpha = 2/(3+1) = 0.5 -> 0.5*50 + 0.5*100 = 75
        // Slow: alpha = 2/(7+1) = 0.25 -> 0.25*50 + 0.75*100 = 87.5
        assert!((fast.value() - 75.0).abs() < 0.001);
        assert!((slow.value() - 87.5).abs() < 0.001);
        assert!(fast.value() < slow.value());
    }

    #[test]
    fn test_ewma_converges_to_constant() {
        let mut ewma = Ewma::from_span(5, 100.0);

        // Repeatedly add the same value
        for _ in 0..100 {
            ewma.add(80.0);
        }

        // Should converge to 80.0
        assert!((ewma.value() - 80.0).abs() < 0.001);
    }

    #[test]
    fn test_ewma_upward_trend() {
        let mut ewma = Ewma::from_span(3, 50.0);

        // Simulate improving performance: 50 -> 60 -> 70 -> 80
        ewma.add(60.0); // 0.5*60 + 0.5*50 = 55
        ewma.add(70.0); // 0.5*70 + 0.5*55 = 62.5
        ewma.add(80.0); // 0.5*80 + 0.5*62.5 = 71.25

        assert!((ewma.value() - 71.25).abs() < 0.001);
        assert!(ewma.value() > 50.0); // Trending up
    }

    #[test]
    fn test_ewma_downward_trend() {
        let mut ewma = Ewma::from_span(3, 100.0);

        // Simulate declining performance: 100 -> 90 -> 80 -> 70
        ewma.add(90.0); // 0.5*90 + 0.5*100 = 95
        ewma.add(80.0); // 0.5*80 + 0.5*95 = 87.5
        ewma.add(70.0); // 0.5*70 + 0.5*87.5 = 78.75

        assert!((ewma.value() - 78.75).abs() < 0.001);
        assert!(ewma.value() < 100.0); // Trending down
    }

    #[test]
    fn test_ewma_with_percentage_values() {
        let mut ewma = Ewma::from_span(7, 100.0);

        // Simulate daily achievement percentages
        let daily_scores = vec![100.0, 95.0, 100.0, 80.0, 90.0, 100.0, 85.0];

        for score in daily_scores {
            ewma.add(score);
        }

        // Should be somewhere between min (80) and max (100)
        assert!(ewma.value() >= 80.0 && ewma.value() <= 100.0);
    }
}
