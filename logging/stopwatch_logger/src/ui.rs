use crossterm::{
    cursor::MoveTo,
    style::Print,
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use std::io::{self, Write};

pub fn print_status(status: &str) {
    let mut stdout = io::stdout();
    let _ = stdout.execute(MoveTo(0, 3));
    let _ = stdout.execute(Clear(ClearType::CurrentLine));
    let _ = stdout.execute(Print(status));
    let _ = stdout.flush();
}

pub fn clear_screen() {
    print!("\x1B[2J\x1B[1;1H");
}

pub fn update_state(is_running: bool) {
    if is_running {
        print_status("Running");
    } else {
        print_status("Stopped");
    }
}

pub fn print_elapsed_time(elapsed: chrono::Duration) {
    let mut stdout = io::stdout();
    let _ = stdout.execute(MoveTo(0, 1));
    let _ = stdout.execute(Clear(ClearType::CurrentLine));
    let _ = stdout.execute(Print(format!("Elapsed time: {}", format_duration(elapsed))));
    let _ = stdout.flush();
}

pub fn format_duration(duration: chrono::Duration) -> String {
    let total_seconds = duration.num_seconds();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
