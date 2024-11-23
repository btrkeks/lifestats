pub fn seconds_to_time_formatter(total_sec: &i64) -> String {
    // Pre-allocate a fixed buffer since we know the exact size needed (5 chars: "HH:MM")
    const FORMATTER: [u8; 5] = [b'0'; 5];
    let mut buf = FORMATTER;

    // Integer division optimizations
    let mins = (total_sec / 60) % 60;
    let hours = total_sec / 3600;

    // Fast byte manipulation for hours
    let h1 = (hours / 10) as u8 + b'0';
    let h2 = (hours % 10) as u8 + b'0';

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