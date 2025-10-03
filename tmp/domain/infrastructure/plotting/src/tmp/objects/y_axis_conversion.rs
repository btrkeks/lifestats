pub type FormatterFn = fn(&i64) -> String;

pub enum YAxisConversion {
    None,
    SecondsToTime,
}

impl YAxisConversion {
    pub fn get_ticks(&self, y_axis_min: i64, y_axis_max: i64, plot_height: usize) -> Vec<i64> {
        match self {
            Self::SecondsToTime => calculate_seconds_ticks(y_axis_min, y_axis_max, plot_height),
            Self::None => calculate_int_ticks(y_axis_min, y_axis_max, plot_height),
        }
    }

    pub fn get_formatter(&self) -> &dyn Fn(&i64) -> String {
        match self {
            YAxisConversion::None => &int_to_string_formatter,
            YAxisConversion::SecondsToTime => &seconds_to_time_formatter,
        }
    }
}

fn get_max_ticks_for_height(plot_height: usize) -> usize {
    plot_height / 20_usize
}

fn get_ticks_for_step_size(y_min: i64, y_max: i64, step_size: i64) -> Vec<i64> {
    let first_tick = y_min + step_size - (y_min % step_size);
    debug_assert!(first_tick % step_size == 0);
    (first_tick..=y_max).step_by(step_size as usize).collect()
}

fn round_up_to_multiple(num: i64, multiple: i64) -> i64 {
    ((num + multiple - 1) / multiple) * multiple
}

pub fn calculate_seconds_ticks(y_min: i64, y_max: i64, plot_height: usize) -> Vec<i64> {
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

pub fn calculate_int_ticks(y_min: i64, y_max: i64, plot_height: usize) -> Vec<i64> {
    let diff = y_max - y_min;

    let aim_number_of_ticks = get_max_ticks_for_height(plot_height) as i64;
    let corresp_step_size = diff / aim_number_of_ticks;

    let digits = corresp_step_size.ilog10();
    let good_multiple = (10_i64.pow(digits) / 4).max(1);
    let step_size = round_up_to_multiple(corresp_step_size, good_multiple);

    get_ticks_for_step_size(y_min, y_max, step_size)
}

pub fn seconds_to_time_formatter(total_sec: &i64) -> String {
    // Pre-allocate a fixed buffer since we know the exact size needed (5 chars: "HH:MM")
    const FORMATTER: [u8; 5] = [b'0'; 5];
    let mut buf = FORMATTER;

    // Integer division optimizations
    let mins = (total_sec / 60) % 60;
    let hours = total_sec / 3600;

    // Fast byte manipulation for hours
    let h1 = (hours / 10) as u8 + b'0';
    let h2 = ((hours % 10) % 24) as u8 + b'0'; // TODO

    // Fast byte manipulation for minutes
    let m1 = (mins / 10) as u8 + b'0';
    let m2 = (mins % 10) as u8 + b'0';

    // Direct buffer writes avoid format! overhead
    buf[0] = h1;
    buf[1] = h2;
    buf[2] = b':';
    buf[3] = m1;
    buf[4] = m2;

    // Safe conversion since we know our buffer is valid UTF-8
    unsafe { String::from_utf8_unchecked(buf.to_vec()) }
}

pub fn int_to_string_formatter(num: &i64) -> String {
    num.to_string()
}
