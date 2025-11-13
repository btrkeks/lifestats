use crate::services::daily_achievement_evaluator::DayEval;
use lifestats_core::shared::percentage::Percentage;

/// Computes the mean percentage achieved over all days with a goal.
///
/// - Only `DayEval` entries with `percentage_achieved = Some(_)` are counted.
/// - Returns `None` if there are no such days (no goals in range).
///
/// # Examples
///
/// ```ignore
/// let evals = vec![
///     DayEval { has_goal: true, percentage_achieved: Some(Percentage::new(100)), .. },
///     DayEval { has_goal: true, percentage_achieved: Some(Percentage::new(80)), .. },
///     DayEval { has_goal: false, percentage_achieved: None, .. },
/// ];
/// let mean = compute_mean_percentage(&evals);
/// assert_eq!(mean, Some(Percentage::new(90))); // (100 + 80) / 2 = 90
/// ```
pub fn compute_mean_percentage(evals: &[DayEval]) -> Option<Percentage> {
    let mut total: u32 = 0;
    let mut count: u32 = 0;

    for eval in evals {
        if let Some(p) = eval.percentage_achieved {
            total += p.as_num() as u32;
            count += 1;
        }
    }

    if count == 0 {
        return None;
    }

    // Integer mean with simple half-up rounding.
    let mean = (total + count / 2) / count;
    Some(Percentage::new(mean as u8))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_compute_mean_percentage_empty_slice() {
        let evals: Vec<DayEval> = vec![];
        assert_eq!(compute_mean_percentage(&evals), None);
    }

    #[test]
    fn test_compute_mean_percentage_all_none() {
        let evals = vec![
            DayEval {
                date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                has_goal: false,
                achieved: false,
                percentage_achieved: None,
            },
            DayEval {
                date: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                has_goal: false,
                achieved: false,
                percentage_achieved: None,
            },
        ];
        assert_eq!(compute_mean_percentage(&evals), None);
    }

    #[test]
    fn test_compute_mean_percentage_single_entry() {
        let evals = vec![DayEval {
            date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            has_goal: true,
            achieved: true,
            percentage_achieved: Some(Percentage::new(100)),
        }];
        assert_eq!(compute_mean_percentage(&evals), Some(Percentage::new(100)));
    }

    #[test]
    fn test_compute_mean_percentage_multiple_entries() {
        let evals = vec![
            DayEval {
                date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                has_goal: true,
                achieved: true,
                percentage_achieved: Some(Percentage::new(100)),
            },
            DayEval {
                date: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                has_goal: true,
                achieved: false,
                percentage_achieved: Some(Percentage::new(50)),
            },
            DayEval {
                date: NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
                has_goal: true,
                achieved: true,
                percentage_achieved: Some(Percentage::new(100)),
            },
        ];
        // (100 + 50 + 100) / 3 = 250 / 3 = 83.33... ≈ 83
        assert_eq!(compute_mean_percentage(&evals), Some(Percentage::new(83)));
    }

    #[test]
    fn test_compute_mean_percentage_with_mixed_none_and_some() {
        let evals = vec![
            DayEval {
                date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                has_goal: false,
                achieved: false,
                percentage_achieved: None,
            },
            DayEval {
                date: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                has_goal: true,
                achieved: true,
                percentage_achieved: Some(Percentage::new(100)),
            },
            DayEval {
                date: NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
                has_goal: true,
                achieved: true,
                percentage_achieved: Some(Percentage::new(80)),
            },
        ];
        // Only count days with Some: (100 + 80) / 2 = 90
        assert_eq!(compute_mean_percentage(&evals), Some(Percentage::new(90)));
    }

    #[test]
    fn test_compute_mean_percentage_rounding_half_up() {
        // Test half-up rounding: 84.5 -> 85
        let evals = vec![
            DayEval {
                date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                has_goal: true,
                achieved: true,
                percentage_achieved: Some(Percentage::new(84)),
            },
            DayEval {
                date: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                has_goal: true,
                achieved: true,
                percentage_achieved: Some(Percentage::new(85)),
            },
        ];
        // (84 + 85) / 2 = 169 / 2 = 84.5
        // With half-up: (169 + 2/2) / 2 = (169 + 1) / 2 = 170 / 2 = 85
        assert_eq!(compute_mean_percentage(&evals), Some(Percentage::new(85)));
    }

    #[test]
    fn test_compute_mean_percentage_over_achievement() {
        let evals = vec![
            DayEval {
                date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                has_goal: true,
                achieved: true,
                percentage_achieved: Some(Percentage::new(150)),
            },
            DayEval {
                date: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                has_goal: true,
                achieved: true,
                percentage_achieved: Some(Percentage::new(200)),
            },
        ];
        // (150 + 200) / 2 = 175
        assert_eq!(compute_mean_percentage(&evals), Some(Percentage::new(175)));
    }

    #[test]
    fn test_compute_mean_percentage_zeros() {
        let evals = vec![
            DayEval {
                date: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                has_goal: true,
                achieved: false,
                percentage_achieved: Some(Percentage::new(0)),
            },
            DayEval {
                date: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
                has_goal: true,
                achieved: false,
                percentage_achieved: Some(Percentage::new(0)),
            },
        ];
        // (0 + 0) / 2 = 0
        assert_eq!(compute_mean_percentage(&evals), Some(Percentage::new(0)));
    }
}
