# Lifestats Stopwatch Service

A DBus-based service library for monitoring the lifestats stopwatch state. This crate provides a client library for listening to stopwatch updates via DBus signals.

## Overview

The stopwatch logger now publishes its state to the DBus session bus at:
- **Service**: `com.lifestats.Stopwatch`
- **Path**: `/com/lifestats/Stopwatch`
- **Interface**: `com.lifestats.Stopwatch1`

This allows external applications (taskbar widgets, monitors, etc.) to display the current stopwatch status in real-time.

## Usage

### As a Library

```rust
use lifestats_stopwatch_service::StopwatchClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = StopwatchClient::new().await?;

    // Get current status (one-time poll)
    if let Some(status) = client.get_status().await? {
        println!("{} - {}", status.statistic_name, status.format_duration());
    }

    // Listen for real-time updates
    client.listen_for_updates(|status| {
        println!("{} - {}", status.statistic_name, status.format_duration());
    }).await?;

    Ok(())
}
```

### Running Examples

```bash
# Poll status every second (simple polling mode)
cargo run -p lifestats-stopwatch-service --example poll_status

# Listen for real-time DBus signals
cargo run -p lifestats-stopwatch-service --example listen_updates

# Taskbar widget (prints status once for use in i3blocks, polybar, etc.)
cargo run -p lifestats-stopwatch-service --example taskbar_widget
```

## Testing the Implementation

### Terminal 1: Start the stopwatch logger
```bash
cargo run -p lifestats-stopwatch-logger
```

This will:
1. Prompt you to select a TimeSpans statistic
2. Start the stopwatch UI
3. Automatically publish state to DBus (unless `DISABLE_DBUS=1` is set)

### Terminal 2: Monitor via polling
```bash
cargo run -p lifestats-stopwatch-service --example poll_status
```

### Terminal 3: Monitor via signals (real-time)
```bash
cargo run -p lifestats-stopwatch-service --example listen_updates
```

## Taskbar Integration

### i3blocks

Add to `~/.config/i3blocks/config`:

```ini
[lifestats-stopwatch]
command=/path/to/target/debug/examples/taskbar_widget
interval=1
```

### Polybar

Add to `~/.config/polybar/config`:

```ini
[module/lifestats-stopwatch]
type = custom/script
exec = /path/to/target/debug/examples/taskbar_widget
interval = 1
```

### Waybar

Add to `~/.config/waybar/config`:

```json
{
  "custom/lifestats-stopwatch": {
    "exec": "/path/to/target/debug/examples/taskbar_widget",
    "interval": 1
  }
}
```

## API

### StopwatchClient

- `new() -> Result<Self>` - Create a new client connected to the session bus
- `get_status() -> Result<Option<StopwatchStatus>>` - Get current status (returns None if service not running)
- `listen_for_updates<F>(callback: F) -> Result<()>` - Listen for real-time updates via DBus signals

### StopwatchStatus

- `statistic_name: String` - Name of the statistic being tracked
- `is_running: bool` - Whether the stopwatch is currently running
- `elapsed_seconds: i64` - Total elapsed time in seconds
- `format_duration() -> String` - Format elapsed time as HH:MM:SS
- `status_string() -> String` - Human-readable status string

## Architecture Notes

The implementation follows hexagonal architecture:

1. **Domain**: `HeartbeatSession` (in `lifestats-application`) manages stopwatch state
2. **Port**: State updates are published via channels
3. **Adapters**:
   - `TerminalInterface` - User input/UI
   - `DBusStatePublisher` - Broadcasts state to DBus
   - `StopwatchClient` (this crate) - Consumes DBus state

The DBus publisher is a secondary adapter that runs alongside the terminal UI, publishing state changes to any interested listeners.

## Disabling DBus

Set the `DISABLE_DBUS=1` environment variable to disable DBus publishing:

```bash
DISABLE_DBUS=1 cargo run -p lifestats-stopwatch-logger
```
