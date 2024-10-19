// db_events/src/lib.rs

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

const EVENT_SERVER_ADDRESS: &str = "127.0.0.1:8081";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Event {
    RowUpdated(u32), // statistic_id
}

pub fn listen_for_db_event<F>(mut handler: F)
where
    F: FnMut(Event) + Send + 'static,
{
    let listener = TcpListener::bind(EVENT_SERVER_ADDRESS).expect("Failed to bind to address");
    println!("Event server listening on {}", EVENT_SERVER_ADDRESS);

    for stream in listener.incoming() {
        let stream = stream.expect("Failed to accept connection");
        let reader = BufReader::new(stream);

        for line in reader.lines() {
            let line = line.expect("Failed to read line");
            let event: Event = serde_json::from_str(&line).expect("Failed to parse event");
            handler(event);
        }
    }
}

pub fn publish_db_event(event: Event) {
    if let Ok(mut stream) = TcpStream::connect(EVENT_SERVER_ADDRESS) {
        if let Ok(json) = serde_json::to_string(&event) {
            let _ = writeln!(stream, "{}", json);
        }
    } else {
        eprintln!("Couldn't connect to db event handler");
    }
}

// use zmq::{Context, Socket};
// use std::thread;
// use std::sync::{Arc, Mutex, Once};
// use anyhow::{Result, Context as _};
//
// static INIT: Once = Once::new();
// static mut CONTEXT: Option<Arc<Context>> = None;
// static mut SOCKET: Option<Arc<Mutex<Socket>>> = None;
//
// const PROXY_FRONT_ADDRESS: &str = "ipc:///tmp/db_events_front.ipc";
// const PROXY_BACK_ADDRESS: &str = "ipc:///tmp/db_events_back.ipc";
//
// fn initialize() -> Result<()> {
//     INIT.call_once(|| {
//         let context = Context::new();
//         let socket = context.socket(zmq::PAIR).expect("Failed to create ZMQ PAIR socket");
//         socket.connect(PROXY_FRONT_ADDRESS).expect("Failed to connect to proxy front");
//
//         unsafe {
//             CONTEXT = Some(Arc::new(context));
//             SOCKET = Some(Arc::new(Mutex::new(socket)));
//         }
//     });
//
//     Ok(())
// }
//
// pub fn publish_db_event(event: Event) -> Result<()> {
//     initialize()?;
//
//     let socket = unsafe { SOCKET.as_ref().unwrap().clone() };
//     let message = serde_json::to_string(&event).context("Failed to serialize event")?;
//
//     socket.lock().unwrap().send(&message, 0).context("Failed to send event")?;
//
//     Ok(())
// }
//
// pub fn listen_for_db_event<F>(mut handler: F) -> Result<()>
// where
//     F: FnMut(Event) + Send + 'static,
// {
//     initialize()?;
//
//     let socket = unsafe { SOCKET.as_ref().unwrap().clone() };
//
//     loop {
//         let message = socket.lock().unwrap().recv_string(0).context("Failed to receive message")?;
//         let event: Event = serde_json::from_str(&message.unwrap()).context("Failed to deserialize event")?;
//         handler(event);
//     }
// }

pub fn run_proxy() -> Result<()> {
    // let context = Context::new();
    // let frontend = context.socket(zmq::XPUB).context("Failed to create XPUB socket")?;
    // let backend = context.socket(zmq::XSUB).context("Failed to create XSUB socket")?;
    //
    // frontend.bind(PROXY_FRONT_ADDRESS).context("Failed to bind frontend socket")?;
    // backend.bind(PROXY_BACK_ADDRESS).context("Failed to bind backend socket")?;
    //
    // println!("Proxy running...");
    // zmq::proxy(&frontend, &backend).context("Failed to run proxy")?;

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::sync::mpsc;
//     use std::time::Duration;
//
//     #[test]
//     fn test_publish_and_listen() {
//         // Start the proxy in a separate thread
//         thread::spawn(|| {
//             run_proxy().unwrap();
//         });
//
//         // Give some time for the proxy to start
//         std::thread::sleep(Duration::from_millis(100));
//
//         let (tx, rx) = mpsc::channel();
//
//         listen_for_db_event(move |event| {
//             tx.send(event).unwrap();
//         }).unwrap();
//
//         // Give some time for the subscriber to connect
//         std::thread::sleep(Duration::from_millis(100));
//
//         let test_event = Event::RowUpdated(42);
//         publish_db_event(test_event.clone()).unwrap();
//
//         // Wait for the event to be received
//         let received_event = rx.recv_timeout(Duration::from_secs(1)).unwrap();
//
//         match (test_event, received_event) {
//             (Event::RowUpdated(sent_id), Event::RowUpdated(received_id)) => {
//                 assert_eq!(sent_id, received_id);
//             }
//             _ => panic!("Received event does not match sent event"),
//         }
//     }
// }
