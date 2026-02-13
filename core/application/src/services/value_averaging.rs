//! Pure functions for computing value averages.
//!
//! These functions operate on pre-fetched data and perform no I/O.
//! All inputs are explicit - no hidden state or time dependencies.

/// Computes the arithmetic mean of i64 values.
/// Returns None if the input slice is empty.
pub fn compute_average_i64(values: &[i64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let sum: i64 = values.iter().sum();
    Some(sum as f64 / values.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_none_for_empty_slice() {
        assert_eq!(compute_average_i64(&[]), None);
    }

    #[test]
    fn returns_value_for_single_element() {
        assert_eq!(compute_average_i64(&[100]), Some(100.0));
    }

    #[test]
    fn computes_mean_of_multiple_values() {
        // (100 + 200 + 300) / 3 = 200
        assert_eq!(compute_average_i64(&[100, 200, 300]), Some(200.0));
    }

    #[test]
    fn handles_fractional_result() {
        // (10 + 20) / 2 = 15.0
        assert_eq!(compute_average_i64(&[10, 20]), Some(15.0));
    }

    #[test]
    fn handles_negative_values() {
        // (-10 + 10) / 2 = 0.0
        assert_eq!(compute_average_i64(&[-10, 10]), Some(0.0));
    }
}
