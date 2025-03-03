use anyhow::Result;
use transfer_objects::raw_goal::RawGoal;

// TODO: Simplify, i.e. probably remove the _opt functions
// TODO: set and get extra data don't seem to belong here
pub trait Goal<T>: Clone {
    fn new_with_target(target: T) -> Self;

    // fn set_target(self, target: T) -> Self;
    fn set_extra_data_1(self, extra_data_1: &str) -> Result<Self>;
    fn get_extra_data_1(&self) -> Option<String>;

    fn get_value(&self) -> &T;

    /// Returns the percentage of value towards the goal. Can be greater than 100%
    ///
    /// None represents that there has nothing been logged.
    /// Usually this means that the default value of T will be used (e.g. 0 for integers or false for booleans),
    /// but there are also cases where there is no default value that makes sense (e.g. time).
    /// In this case the function will just return 0.0.
    fn get_percentage_achieved(&self, value: Option<&T>) -> f64;

    fn is_achieved(&self, value: Option<&T>) -> bool {
        self.get_percentage_achieved(value) >= 1.0
    }

    // Returns the percentage towards the goal capped by 100%
    fn calculate_score(&self, value: Option<&T>) -> f64 {
        self.get_percentage_achieved(value).min(1.0)
    }

    // TODO: Should probably be removed
    // /// Calculates the mean score across a series of optional values.
    // ///
    // /// The mean score is computed from all non-None values, where each value's score
    // /// is calculated using `calculate_score()` (capped at 100%).
    // ///
    // /// # Arguments
    // ///
    // /// * `values` - A slice of optional values (`&[Option<T>]`) representing progress towards the goal.
    // ///
    // /// # Returns
    // ///
    // /// * `Some(f64)` - The mean score if there are any non-None values.
    // /// * `None` - If the slice is empty or contains only None values.
    // fn calculate_mean_score(&self, values: &[Option<T>]) -> Option<f64> {
    //     let (sum, count) = values
    //         .iter()
    //         .filter_map(|value| self.calculate_score(value))
    //         .fold((0.0, 0), |acc, x| (acc.0 + x, acc.1 + 1));
    //
    //     (count > 0).then_some(sum / count as f64)
    // }

    fn into_raw_goal(self) -> RawGoal;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestGoal;

    impl Goal<u32> for TestGoal {
        fn new_with_target(_target: u32) -> Self {
            unimplemented!()
        }

        fn set_extra_data_1(self, _extra_data_1: &str) -> Result<Self> {
            unimplemented!()
        }

        fn get_extra_data_1(&self) -> Option<String> {
            unimplemented!()
        }

        fn get_value(&self) -> &u32 {
            unimplemented!()
        }

        fn get_percentage_achieved(&self, value: Option<&u32>) -> f64 {
            let value = value.unwrap_or(&0);

            *value as f64 / 10.0
        }

        fn into_raw_goal(self) -> RawGoal {
            unimplemented!()
        }
    }

    #[test]
    fn test_score_is_capped_by_one() {
        let test_goal = TestGoal;
        let value = 20;
        assert_eq!(test_goal.calculate_score(Some(&value)), 1.0);
    }

    // #[test]
    // fn test_mean_score_normal_values() {
    //     let test_goal = TestGoal;
    //     let values = [Some(5), Some(10), Some(15)];
    //     assert_eq!(
    //         test_goal.calculate_mean_score(&values),
    //         Some((0.5 + 1.0 + 1.0) / 3.0)
    //     );
    // }
    //
    // #[test]
    // fn test_mean_score_with_nones() {
    //     let test_goal = TestGoal;
    //     let values = [Some(10), None, Some(20)];
    //     assert_eq!(test_goal.calculate_mean_score(&values), Some(1.0));
    // }
    //
    // #[test]
    // fn test_mean_score_empty_list() {
    //     let test_goal = TestGoal;
    //     let values: [Option<u32>; 0] = [];
    //     assert_eq!(test_goal.calculate_mean_score(&values), None);
    // }
    //
    // #[test]
    // fn test_mean_score_all_nones() {
    //     let test_goal = TestGoal;
    //     let values = [None, None, None];
    //     assert_eq!(test_goal.calculate_mean_score(&values), None);
    // }
    //
    // #[test]
    // fn test_mean_score_all_exceed_goal() {
    //     let test_goal = TestGoal;
    //     let values = [Some(15), Some(20), Some(30)];
    //     assert_eq!(test_goal.calculate_mean_score(&values), Some(1.0));
    // }

    #[test]
    fn test_none_value_gives_zero_score() {
        let test_goal = TestGoal;
        let value = None;
        assert_eq!(test_goal.calculate_score(value), 0.0);
    }
}
