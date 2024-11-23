use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::time::Duration;
use crate::statistics::objects::StatisticId;

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

#[cfg(test)]
mod tests {
    // use super::*;
    // use std::sync::{Arc, Mutex};
    // use std::thread;
    // use std::time::Duration;

    // TODO: Need to use different socket for tests, as they interfere currently (esp with real running one)
    // #[test]
    // fn test_event_publishing_and_receiving() {
    //     // Create a shared vector to store received events
    //     let received_events = Arc::new(Mutex::new(Vec::new()));
    //     let received_events_clone = received_events.clone();
    //
    //     // Start the listener in a separate thread
    //     let _ = thread::spawn(move || {
    //         listen_for_db_event(move |event| {
    //             received_events_clone.lock().unwrap().push(event);
    //         });
    //     });
    //
    //     // Give the listener some time to start up
    //     thread::sleep(Duration::from_millis(100));
    //
    //     // Test events
    //     let test_events = vec![
    //         Event::ValueUpdated(StatisticId(1)),
    //         Event::GoalUpdated(StatisticId(2)),
    //         Event::ValueUpdated(StatisticId(3)),
    //     ];
    //
    //     // Publish test events
    //     for event in test_events.iter() {
    //         publish_db_event(event.clone());
    //     }
    //
    //     // Give some time for events to be processed
    //     thread::sleep(Duration::from_millis(100));
    //
    //     // Check received events
    //     let received = received_events.lock().unwrap();
    //     assert_eq!(received.len(), 3);
    //
    //     for (sent, received) in test_events.iter().zip(received.iter()) {
    //         match (sent, received) {
    //             (Event::ValueUpdated(sid1), Event::ValueUpdated(sid2)) => {
    //                 assert_eq!(sid1, sid2);
    //             },
    //             (Event::GoalUpdated(sid1), Event::GoalUpdated(sid2)) => {
    //                 assert_eq!(sid1, sid2);
    //             },
    //             _ => panic!("Event types don't match"),
    //         }
    //     }
    // }

    // #[test]
    // fn test_multiple_publishers() {
    //     let received_events = Arc::new(Mutex::new(Vec::new()));
    //     let received_events_clone = received_events.clone();
    //
    //     // Start listener
    //     let _ = thread::spawn(move || {
    //         listen_for_db_event(move |event| {
    //             received_events_clone.lock().unwrap().push(event);
    //         });
    //     });
    //
    //     thread::sleep(Duration::from_millis(100));
    //
    //     // Spawn multiple publisher threads
    //     let mut publisher_threads = vec![];
    //     for i in 0..5 {
    //         let handle = thread::spawn(move || {
    //             publish_db_event(Event::ValueUpdated(StatisticId(i)));
    //         });
    //         publisher_threads.push(handle);
    //     }
    //
    //     // Wait for all publishers to complete
    //     for handle in publisher_threads {
    //         handle.join().unwrap();
    //     }
    //
    //     thread::sleep(Duration::from_millis(100));
    //
    //     // Verify we received all events
    //     let received = received_events.lock().unwrap();
    //     assert_eq!(received.len(), 5);
    // }
}