pub fn seconds_to_time_formatter(total_sec: &i64) -> String {
    let mins = (total_sec / 60) % 60;
    let hours = total_sec / 3600;
    format!("{hours:02}:{mins:02}")
}

pub fn int_to_string_formatter(num: &i64) -> String {
    num.to_string()
}

fn get_max_ticks_for_height(plot_height: u32) -> u32 {
    plot_height / 20
}

fn get_ticks_for_step_size(y_min: i64, y_max: i64, step_size: i64) -> Vec<i64> {
    let first_tick = y_min + step_size - (y_min % step_size);
    debug_assert!(first_tick % step_size == 0);
    (first_tick..=y_max).step_by(step_size as usize).collect()
}

fn round_up_to_multiple(num: i64, multiple: i64) -> i64 {
    ((num + multiple - 1) / multiple) * multiple
}

pub fn calculate_seconds_ticks(y_min: i64, y_max: i64, plot_height: u32) -> Vec<i64> {
    let diff = y_max - y_min;

    let aim_number_of_ticks = get_max_ticks_for_height(plot_height) as i64;
    let corresp_step_size = diff / aim_number_of_ticks;

    let multiple: i64 = match corresp_step_size {
        ..=300 => 5 * 60, // <= 5 min
        301..=900 => 900, // <= 15 min
        _ => 1800,
    };
    let step_size = round_up_to_multiple(corresp_step_size, multiple);

    get_ticks_for_step_size(y_min, y_max, step_size)
}

pub fn calculate_int_ticks(y_min: i64, y_max: i64, plot_height: u32) -> Vec<i64> {
    let diff = y_max - y_min;

    let aim_number_of_ticks = get_max_ticks_for_height(plot_height) as i64;
    let corresp_step_size = diff / aim_number_of_ticks;

    let digits = corresp_step_size.ilog10();
    let good_multiple = 10_i64.pow(digits) / 4;
    let step_size = round_up_to_multiple(corresp_step_size, good_multiple);

    get_ticks_for_step_size(y_min, y_max, step_size)
}
