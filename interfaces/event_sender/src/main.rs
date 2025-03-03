use anyhow::Result;
use clap::Parser;
use events::event_port::{Event, EventPublisher};
use events::unix_socket::UnixSocketEvent;
use log::{error, info};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

/// Emits a NewDay event through the Unix socket event system
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the Unix socket
    #[arg(short, long, default_value = "/tmp/events.socket")]
    socket: PathBuf,

    /// Maximum time to wait for the event to be delivered (in milliseconds)
    #[arg(short, long, default_value = "1000")]
    timeout: u64,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    info!("Emitting NewDay event to socket: {}", args.socket.display());

    let event_publisher = UnixSocketEvent::new(args.socket);
    event_publisher.publish_event_sync(Event::NewDay)
}