use crate::event_port::{Event, EventHandler, EventPublisher, EventSubscriber};
use anyhow::Result;
use log::info;
use std::fs;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;
use std::time::Duration;

pub struct UnixSocketEvent {
    socket_path: PathBuf,
}

impl UnixSocketEvent {
    pub fn new(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }
}

impl Default for UnixSocketEvent {
    fn default() -> Self {
        Self {
            socket_path: PathBuf::from("/tmp/events.socket"),
        }
    }
}

impl EventPublisher for UnixSocketEvent {
    fn publish_event_async(&self, event: Event) {
        // TODO: This should be asynchronous
        //       The problem is, that the program might exit immediately after calling this function
        //       so the event might not be sent if just spawning a thread.
        //       An idea would be to wait for the thread in the drop function.
        if let Err(e) = attempt_publish(&self.socket_path, event) {
            log::error!("Failed to publish event: {}", e);
        }
    }

    fn publish_event_sync(&self, event: Event) -> Result<()> {
        attempt_publish(&self.socket_path, event)
    }
}

impl EventSubscriber for UnixSocketEvent {
    fn listen_for_events(&self, event_handler: impl EventHandler) -> Result<JoinHandle<()>> {
        // Remove the existing socket if it still exists
        // Otherwise we will get an error when trying to bind
        if self.socket_path.exists() {
            fs::remove_file(&self.socket_path)?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;

        info!("Listening for events on {}", self.socket_path.display());
        Ok(std::thread::spawn(move || {
            let mut event_handler = event_handler;

            for stream in listener.incoming() {
                let stream = stream.expect("Failed to accept connection");
                handle_unix_message(&mut event_handler, stream);
            }
        }))
    }
}

fn handle_unix_message(event_handler: &mut impl EventHandler, stream: UnixStream) {
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        if let Ok(line) = line {
            if let Ok(event) = serde_json::from_str(&line) {
                event_handler.handle_event(event);
            } else {
                eprintln!("Failed to deserialize json: {}", line);
            }
        } else {
            eprintln!("Failed to read line");
        }
    }
}

fn attempt_publish(socket_path: &Path, event: Event) -> Result<()> {
    let mut stream = UnixStream::connect(socket_path)?;
    stream.set_write_timeout(Some(Duration::from_millis(100)))?;

    let json = serde_json::to_string(&event)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    writeln!(stream, "{}", json)?;
    Ok(())
}

// TODO: Might need to clean up the socket file when the program exits

#[cfg(test)]
mod tests {
    use crate::event_port::{Event, EventHandler, EventPublisher, EventSubscriber};
    use crate::unix_socket::UnixSocketEvent;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::time::Duration;
    use tempfile::TempDir;
    use lifestats_domain::shared::ids::StatisticId;

    #[derive(Debug)]
    struct TestEventHandler {
        received: mpsc::Sender<Event>,
    }

    impl EventHandler for TestEventHandler {
        fn handle_event(&mut self, event: Event) {
            self.received.send(event).unwrap();
        }
    }

    fn get_test_socket_path() -> (TempDir, PathBuf) {
        let temp_dir = tempfile::tempdir().unwrap();
        let socket_path = temp_dir.path().join("test.sock");
        (temp_dir, socket_path)
    }

    #[test]
    fn test_event_delivery() {
        let (_tmp_dir, socket_path) = get_test_socket_path(); // Need to hold tmp_dir
        let event_pub_sub = UnixSocketEvent::new(socket_path);

        // Create channel to receive events
        let (tx, rx) = mpsc::channel();
        let handler = TestEventHandler { received: tx };

        // Start listening in a separate thread
        let _listener = event_pub_sub.listen_for_events(handler).unwrap();

        // Give the listener time to start
        std::thread::sleep(Duration::from_millis(100));

        // Publish some test events
        let test_events = vec![
            Event::ValueUpdated(StatisticId(1)),
            Event::GoalUpdated(StatisticId(2)),
        ];

        for event in test_events.iter() {
            event_pub_sub.publish_event_async(event.clone());
        }

        // Verify events were received in order
        for expected in test_events {
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(received) => assert_eq!(received, expected),
                Err(_) => panic!("Didn't receive expected event: {:?}", expected),
            }
        }
    }

    #[test]
    fn test_reconnection() {
        let (_tmp_dir, socket_path) = get_test_socket_path(); // Need to hold tmp_dir
        let event_pub_sub = UnixSocketEvent::new(socket_path);

        let (tx, rx) = mpsc::channel();
        let handler = TestEventHandler { received: tx };

        // Start listener
        let listener = event_pub_sub.listen_for_events(handler).unwrap();

        // Send initial event
        event_pub_sub.publish_event_async(Event::ValueUpdated(StatisticId(1)));
        assert!(rx.recv_timeout(Duration::from_secs(1)).is_ok());

        // Stop listener
        drop(listener);

        // Create new listener
        let (tx2, rx2) = mpsc::channel();
        let handler2 = TestEventHandler { received: tx2 };
        let _listener2 = event_pub_sub.listen_for_events(handler2).unwrap();

        // Send event after reconnection
        event_pub_sub.publish_event_async(Event::ValueUpdated(StatisticId(2)));
        assert!(rx2.recv_timeout(Duration::from_secs(1)).is_ok());
    }
}
