// Diagnostic of the reviewed implementation, not a regression-test expectation.
use lifestats_core::shared::time_frame_list::{TimeFrame, TimeFrameList};
use lifestats_core::statistic::value_objects::goals::{amount_goal::AmountGoal, goal_trait::Goal};
use lifestats_core::statistic::value_objects::update_policies::{
    time_frame_list::TimeFrameListUpdatePolicy, update_policy_trait::UpdatePolicy,
};

fn main() {
    let goal = AmountGoal::new(10_i64, true);
    let score = goal.get_percentage_achieved(Some(&30_i64)).as_num();
    let existing: TimeFrameList = r#"[{"start":"09:00","end":"10:00"}]"#.parse().unwrap();
    let next: TimeFrameList = r#"[{"start":"11:00","end":"12:00"}]"#.parse().unwrap();
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let append_panics = std::panic::catch_unwind(|| {
        TimeFrameListUpdatePolicy::Append.apply(Some(existing), next)
    }).is_err();
    std::panic::set_hook(hook);
    let reversed_accepted = serde_json::from_str::<TimeFrame>(
        r#"{"start":"12:00","end":"09:00"}"#,
    ).is_ok();
    let overlapping: TimeFrameList =
        r#"[{"start":"09:00","end":"11:00"},{"start":"10:00","end":"12:00"}]"#.parse().unwrap();
    println!("{}", serde_json::json!({
        "amount_goal_target": 10,
        "amount_value": 30,
        "amount_reported_percentage": score,
        "second_append_panics": append_panics,
        "reversed_interval_deserialization_accepted": reversed_accepted,
        "overlapping_intervals_raw_total_seconds": overlapping.get_total_duration().num_seconds(),
        "scope": "Pure public domain methods, debug build, no database or HTTP. Diagnostic results describe this revision; no production incidence claimed."
    }));
}
