use crate::ui::user_interface_trait::{UserEvent, UserInput, UserInterface};
use anyhow::Result;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode},
    execute, queue,
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use lifestats_application::StatisticMetadata;
use std::io::{self, Write};
use tokio::sync::mpsc;

pub struct TerminalInterface {
    stdout: io::Stdout,
}

impl TerminalInterface {
    pub fn new() -> Self {
        Self {
            stdout: io::stdout(),
        }
    }

    fn format_duration(duration: chrono::Duration) -> String {
        let total_seconds = duration.num_seconds();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }
}

impl Default for TerminalInterface {
    fn default() -> Self {
        Self::new()
    }
}

impl UserInterface for TerminalInterface {
    fn init(&mut self) -> Result<()> {
        enable_raw_mode()?;
        execute!(self.stdout, Hide, Clear(ClearType::All), MoveTo(0, 0))?;
        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        disable_raw_mode()?;
        execute!(self.stdout, Show)?;
        Ok(())
    }

    fn show_stopwatch_header(&mut self, metadata: &StatisticMetadata) -> Result<()> {
        execute!(
            self.stdout,
            Clear(ClearType::All),
            MoveTo(0, 0),
            Print(format!(
                "Stopwatch for '{}'. Press SPACE to pause/resume, 'q' to quit.\r\n",
                metadata.name()
            ))
        )?;
        Ok(())
    }

    fn update_stopwatch_state(
        &mut self,
        is_running: bool,
        elapsed_time: chrono::Duration,
    ) -> Result<()> {
        queue!(
            self.stdout,
            MoveTo(0, 1),
            Clear(ClearType::FromCursorDown),
            Print(format!(
                "Elapsed time: {}\r\n",
                Self::format_duration(elapsed_time)
            )),
            MoveTo(0, 2),
            Print(format!(
                "Status: {}\r\n",
                if is_running { "Running" } else { "Stopped" }
            ))
        )?;
        self.stdout.flush()?;
        Ok(())
    }

    fn show_choose_statistic_dialog(
        &mut self,
        statistics: &[StatisticMetadata],
    ) -> Result<UserInput> {
        queue!(
            self.stdout,
            Clear(ClearType::All),
            MoveTo(0, 0),
            Print("Available statistics:\r\n")
        )?;
        for meta_data in statistics {
            queue!(
                self.stdout,
                Print(format!(
                    "id: {:3}, name: {}\r\n",
                    meta_data.id(),
                    meta_data.name()
                ))
            )?;
        }

        let prompt = "\r\nEnter ID to log (or 'q' to quit): ";
        let prompt_len = prompt.chars().count() - 2; // -2 for \r\n
        queue!(self.stdout, Print(prompt))?;
        self.stdout.flush()?;

        let mut current_input = String::new();
        let input_line = (2 + statistics.len()) as u16;

        loop {
            if let Ok(true) = event::poll(std::time::Duration::from_millis(100)) {
                if let Ok(Event::Key(key_event)) = event::read() {
                    match key_event.code {
                        KeyCode::Char('q') if current_input.is_empty() => {
                            queue!(self.stdout, Print('q'), Print("\r\n"))?;
                            self.stdout.flush()?;
                            return Ok(UserInput::Quit);
                        }
                        KeyCode::Char(c) if c.is_ascii_digit() => {
                            current_input.push(c);
                            queue!(self.stdout, Print(c))?;
                            self.stdout.flush()?;
                        }
                        KeyCode::Enter if !current_input.is_empty() => {
                            queue!(self.stdout, Print("\r\n"))?;
                            self.stdout.flush()?;
                            if let Ok(num) = current_input.parse() {
                                return Ok(UserInput::Number(num));
                            } else {
                                queue!(self.stdout, Print("\r\nInvalid number, try again: "))?;
                                self.stdout.flush()?;
                                current_input.clear();
                            }
                        }
                        KeyCode::Backspace if !current_input.is_empty() => {
                            current_input.pop();
                            queue!(
                                self.stdout,
                                MoveTo(prompt_len as u16 + current_input.len() as u16, input_line),
                                Print(" "),
                                MoveTo(prompt_len as u16 + current_input.len() as u16, input_line)
                            )?;
                            self.stdout.flush()?;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn show_invalid_id_message(&mut self, input: u32) -> Result<()> {
        execute!(
            self.stdout,
            Print(format!("\r\nInvalid input '{}'.\r\n", input))
        )?;
        Ok(())
    }

    fn run_event_loop(&self) -> Result<mpsc::Receiver<UserEvent>> {
        let (tx, rx) = mpsc::channel(32);

        // Spawn keyboard event handler
        tokio::spawn(async move {
            loop {
                if event::poll(std::time::Duration::from_millis(100)).unwrap_or(false) {
                    if let Ok(Event::Key(key_event)) = event::read() {
                        let event = match key_event.code {
                            KeyCode::Char(' ') => Some(UserEvent::Toggle),
                            KeyCode::Char('q') => Some(UserEvent::Quit),
                            _ => None,
                        };

                        if let Some(event) = event {
                            if tx.send(event.clone()).await.is_err()
                                || matches!(event, UserEvent::Quit)
                            {
                                break;
                            }
                        }
                    }
                }
            }
        });

        Ok(rx)
    }
}
