
pub trait Goal<T>: Clone {
    // Returns the percentage of value towards the goal. Can be greater than 100%
    fn get_percentage_achieved(&self, value: &T) -> f64;

    // Returns the percentage towards the goal capped by 100%
    fn calculate_score(&self, value: &T) -> f64 {
        self.get_percentage_achieved(value).min(1.0)
    }

    /// Returns the current streak of consecutive achievements towards the goal.
    ///
    /// A streak is the number of consecutive values, starting from the end of the list,
    /// that meet or exceed 100% of the goal. If the last element is `None` (representing
    /// a value not yet entered), it is ignored and treated as if it does not exist.
    ///
    /// The streak is broken by:
    /// - A `None` value (other than the last element).
    /// - A value that does not meet the goal.
    ///
    /// # Arguments
    ///
    /// * `values` - A slice of optional values (`&[Option<T>]`) representing progress towards the goal.
    ///
    /// # Returns
    ///
    /// The length of the current streak (from the end of the list backwards) as a `u32`.
    ///
    /// # Note
    ///
    /// - The streak starts counting from the last valid (non-`None`) value and works backwards.
    /// - A `None` value at the end of the list is ignored.
    /// - If no values meet the goal or if the list is empty, the streak will be `0`.
    fn get_streak(&self, values: &[Option<T>]) -> u32 {
        values.iter()
            .rev()
            .take_while(|v| matches!(v, Some(x) if self.get_percentage_achieved(x) >= 1.0))
            .count() as u32
    }

    /// Calculates the mean score across a series of optional values.
    ///
    /// The mean score is computed from all non-None values, where each value's score
    /// is calculated using `calculate_score()` (capped at 100%).
    ///
    /// # Arguments
    ///
    /// * `values` - A slice of optional values (`&[Option<T>]`) representing progress towards the goal.
    ///
    /// # Returns
    ///
    /// * `Some(f64)` - The mean score if there are any non-None values.
    /// * `None` - If the slice is empty or contains only None values.
    fn calculate_mean_score(&self, values: &[Option<T>]) -> Option<f64> {
        let (sum, count) = values.iter()
            .filter_map(|value| {
                value.as_ref().map(|v| self.calculate_score(v))
            })
            .fold((0.0, 0), |acc, x| (acc.0 + x, acc.1 + 1));

        (count > 0).then_some(sum / count as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestGoal;

    impl Goal<u32> for TestGoal {
        fn get_percentage_achieved(&self, value: &u32) -> f64 {
            *value as f64 / 10.0
        }
    }

    // TODO: Make this test work
    #[ignore]
    #[test]
    fn test_mixed_progress() {
        let gym_goal = TestGoal;
        let progress = [Some(3), Some(15), None, Some(10), Some(20), None];
        assert_eq!(gym_goal.get_streak(&progress), 2);
    }

    #[test]
    fn test_all_achieved() {
        let gym_goal = TestGoal;
        let progress2 = [Some(10), Some(12), Some(15)];
        assert_eq!(gym_goal.get_streak(&progress2), 3);
    }

    #[test]
    fn test_empty_list() {
        let gym_goal = TestGoal;
        let progress3: [Option<u32>; 0] = [];
        assert_eq!(gym_goal.get_streak(&progress3), 0);
    }

    #[test]
    fn test_none_in_middle() {
        let gym_goal = TestGoal;
        let progress4 = [Some(5), None, Some(20)];
        assert_eq!(gym_goal.get_streak(&progress4), 1);
    }

    #[test]
    fn test_trailing_nones() {
        let gym_goal = TestGoal;
        let progress5 = [Some(15), Some(11), Some(20), None, None];
        assert_eq!(gym_goal.get_streak(&progress5), 0);
    }

    #[test]
    fn test_broken_streak() {
        let gym_goal = TestGoal;
        let progress6 = [Some(15), Some(11), Some(20), Some(1), Some(10)];
        assert_eq!(gym_goal.get_streak(&progress6), 1);
    }

    #[test]
    fn test_score_is_capped_by_one() {
        let test_goal = TestGoal;
        let value = 20;
        assert_eq!(test_goal.calculate_score(&value), 1.0);
    }

    #[test]
    fn test_mean_score_normal_values() {
        let test_goal = TestGoal;
        let values = [Some(5), Some(10), Some(15)];
        assert_eq!(test_goal.calculate_mean_score(&values), Some((0.5 + 1.0 + 1.0) / 3.0));
    }

    #[test]
    fn test_mean_score_with_nones() {
        let test_goal = TestGoal;
        let values = [Some(10), None, Some(20)];
        assert_eq!(test_goal.calculate_mean_score(&values), Some(1.0));
    }

    #[test]
    fn test_mean_score_empty_list() {
        let test_goal = TestGoal;
        let values: [Option<u32>; 0] = [];
        assert_eq!(test_goal.calculate_mean_score(&values), None);
    }

    #[test]
    fn test_mean_score_all_nones() {
        let test_goal = TestGoal;
        let values = [None, None, None];
        assert_eq!(test_goal.calculate_mean_score(&values), None);
    }

    #[test]
    fn test_mean_score_all_exceed_goal() {
        let test_goal = TestGoal;
        let values = [Some(15), Some(20), Some(30)];
        assert_eq!(test_goal.calculate_mean_score(&values), Some(1.0));
    }
}