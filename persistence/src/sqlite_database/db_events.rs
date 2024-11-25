use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::time::Duration;
use crate::statistics::objects::statistic_metadata::StatisticId;

const SOCKET_PATH: &str = "/tmp/stats_events.sock";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Event {
    ValueUpdated(StatisticId),
    GoalUpdated(StatisticId),
}

// TODO: Maybe add an object for the type of socket to listen to (TCP, Domain Socket, ...)
//       Especially for testing / mocking purposes
pub fn listen_for_db_event<F>(event_handler: F)
where
    F: Fn(Event) + Send + Clone + 'static,
{
    // TODO: Is this necessary?
    // Clean up old socket if it exists
    if Path::new(SOCKET_PATH).exists() {
        std::fs::remove_file(SOCKET_PATH).expect("Failed to remove old socket");
    }

    let listener = UnixListener::bind(SOCKET_PATH).expect("Failed to bind to socket");
    println!("Event server listening on {}", SOCKET_PATH);

    for stream in listener.incoming() {
        let stream = stream.expect("Failed to accept connection");
        let handler = event_handler.clone();
        // thread::spawn(move || {
            handle_unix_message(handler, stream);
        // });
    }
}

fn handle_unix_message<F>(event_handler: F, stream: UnixStream)
where
    F: Fn(Event) + Send + 'static
{
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        if let Ok(line) = line {
            if let Ok(event) = serde_json::from_str(&line) {
                event_handler(event);
            } else {
                eprintln!("Failed to deserialize json: {}", line);
            }
        } else {
            eprintln!("Failed to read line");
        }
    }
}

pub fn publish_db_event(event: Event) {
    if let Err(e) = attempt_publish(event) {
        eprintln!("Failed to publish event: {}", e);
    }
}

fn attempt_publish(event: Event) -> std::io::Result<()> {
    let mut stream = UnixStream::connect(SOCKET_PATH)?;
    stream.set_write_timeout(Some(Duration::from_millis(100)))?;

    let json = serde_json::to_string(&event)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    writeln!(stream, "{}", json)?;
    Ok(())
}