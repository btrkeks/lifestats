# Stopwatch Logger Integration Guide

This document explains how the stopwatch logger tracks time and how external programs (like taskbar widgets) can access the current stopwatch state.

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [DBus Interface (Recommended)](#dbus-interface-recommended)
- [Quick Start: Taskbar Integration](#quick-start-taskbar-integration)
- [Timing Mechanisms](#timing-mechanisms)
- [Code Examples](#code-examples)
- [Technical Details](#technical-details)

## Architecture Overview

The stopwatch logger uses a **heartbeat-based architecture** with the following components:

```
┌─────────────────────────────────────────────────────────┐
│ Main Thread (UI)                                        │
│  - Terminal display                                     │
│  - Keyboard input (SPACE = pause/resume, 'q' = quit)   │
└────────────┬────────────────────────────────────────────┘
             │ HeartMessage (Beat/Stop)
             ↓
┌─────────────────────────────────────────────────────────┐
│ Worker Thread (Heart)                                   │
│  - Manages all timing logic                             │
│  - Updates atomic duration counter every 1s             │
│  - Persists to database every flush_interval (default 1m)│
│  - Publishes HeartbeatUpdate events via broadcast       │
└────────────┬────────────────────────────────────────────┘
             │ HeartbeatUpdate (DurationChanged/Started/Stopped)
             ├───────────────────┬─────────────────────────┘
             ↓                   ↓
    ┌────────────────┐   ┌──────────────────┐
    │ Terminal UI    │   │ DBus Publisher   │
    │ (Subscriber)   │   │ (Subscriber)     │
    └────────────────┘   └────────┬─────────┘
                                  │ DBus Signal
                                  ↓
                         ┌─────────────────────┐
                         │ External Programs   │
                         │ (Taskbar Widgets)   │
                         └─────────────────────┘
```

### Key Components

1. **HeartbeatSession**: Public API for starting/stopping the stopwatch
2. **Heart (Worker Thread)**: Background thread managing timing logic
3. **TotalDurationManager**: Thread-safe `AtomicU64` counter for elapsed time
4. **DBusStatePublisher**: Broadcasts state to external programs via DBus

## DBus Interface (Recommended)

The stopwatch exposes its state via DBus, allowing any program to query or monitor the current time.

### DBus Details

- **Service**: `com.lifestats.Stopwatch`
- **Path**: `/com/lifestats/Stopwatch`
- **Interface**: `com.lifestats.Stopwatch1`

### Methods

#### `get_state() -> (String, bool, i64)`

Returns the current stopwatch state:
- `String`: Statistic name (e.g., "Work", "Study")
- `bool`: Is running (true/false)
- `i64`: Elapsed seconds

**Returns empty values** `("", false, 0)` **when no stopwatch is active**.

#### Signal: `state_changed(String, bool, i64)`

Emitted automatically when state changes:
- On stopwatch start
- On stopwatch stop
- Every 1 second while running (duration updates)

### Disabling DBus

Set environment variable `DISABLE_DBUS=1` to run without DBus (for debugging or minimal systems).

## Quick Start: Taskbar Integration

The easiest way to integrate the stopwatch into your taskbar is to use the pre-built client library.

### 1. Build the Taskbar Widget

```bash
cd /path/to/lifestats
cargo build -p lifestats-stopwatch-service --example taskbar_widget --release
```

This creates a binary at: `target/release/examples/taskbar_widget`

### 2. Test It

```bash
# Start the stopwatch logger in one terminal
cargo run -p lifestats-stopwatch-logger

# In another terminal, run the widget
./target/release/examples/taskbar_widget
```

**Output examples:**
```
Work: 01:23:45        # When running
Work: 01:23:45 ⏸     # When paused
(no stopwatch)        # When not running
```

### 3. Configure Your Taskbar

#### i3blocks

Add to `~/.config/i3blocks/config`:

```ini
[lifestats-stopwatch]
command=/path/to/lifestats/target/release/examples/taskbar_widget
interval=1
color=#00FF00
```

#### Polybar

Add to `~/.config/polybar/config`:

```ini
[module/lifestats]
type = custom/script
exec = /path/to/lifestats/target/release/examples/taskbar_widget
interval = 1
format = <label>
format-foreground = #00FF00
```

#### Waybar

Add to `~/.config/waybar/config`:

```json
"custom/lifestats": {
    "exec": "/path/to/lifestats/target/release/examples/taskbar_widget",
    "interval": 1,
    "format": "⏱ {}",
    "return-type": "text"
}
```

## Timing Mechanisms

The stopwatch uses **three timing intervals**:

### 1. Duration Updates (1 second)

- **Purpose**: Update UI and external programs with current elapsed time
- **Mechanism**: Worker thread emits `HeartbeatUpdate::DurationChanged` every 1s
- **Trigger**: DBus `state_changed` signal broadcast

### 2. Flush Interval (Configurable, default 1 minute)

- **Purpose**: Persist progress to database
- **Mechanism**: Worker thread calls `persist_time_frame()` periodically
- **Why**: Balance between data safety and I/O overhead

### 3. Idle Timeout (Configurable, default 1 day)

- **Purpose**: Auto-stop if no heartbeats received
- **Mechanism**: Worker thread checks `last_beat` timestamp
- **Why**: Prevents accidental logging when user forgets to stop

### Atomic Duration Counter

The elapsed time is stored in an `AtomicU64`:

```rust
pub struct TotalDurationManager {
    seconds: AtomicU64,  // Lock-free, thread-safe
}
```

This allows:
- **Non-blocking reads**: UI and DBus can query without waiting
- **Lock-free updates**: Worker thread increments every second
- **Multiple subscribers**: Any number of programs can read simultaneously

## Code Examples

### Example 1: Polling (Simple)

```rust
use lifestats_stopwatch_service::StopwatchClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = StopwatchClient::new().await?;

    loop {
        match client.get_status().await? {
            Some(status) => {
                println!("{}: {} (running: {})",
                    status.statistic_name,
                    status.format_duration(),
                    status.is_running
                );
            }
            None => println!("No stopwatch active"),
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}
```

### Example 2: Signal-Based (Efficient)

```rust
use lifestats_stopwatch_service::StopwatchClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = StopwatchClient::new().await?;

    // This blocks and calls the closure on every update
    client.listen_for_updates(|status| {
        if let Some(s) = status {
            println!("{}: {}", s.statistic_name, s.format_duration());
        } else {
            println!("Stopwatch stopped");
        }
    }).await?;

    Ok(())
}
```

### Example 3: Taskbar Widget (One-shot)

```rust
use lifestats_stopwatch_service::StopwatchClient;

#[tokio::main]
async fn main() {
    let client = match StopwatchClient::new().await {
        Ok(c) => c,
        Err(_) => {
            println!("(no stopwatch)");
            return;
        }
    };

    match client.get_status().await {
        Ok(Some(status)) => {
            let pause_indicator = if status.is_running { "" } else { " ⏸" };
            println!("{}: {}{}",
                status.statistic_name,
                status.format_duration(),
                pause_indicator
            );
        }
        _ => println!("(no stopwatch)"),
    }
}
```

## Technical Details

### Source Files

| File | Purpose |
|------|---------|
| `src/main.rs` | Entry point, UI orchestration |
| `src/dbus/mod.rs` | DBus interface implementation |
| `src/ui/terminal_interface.rs` | Terminal display and keyboard input |
| `../../core/application/src/use_cases/session_tracking_service.rs` | Core session and timing logic |

### Thread Architecture

1. **Main Thread**:
   - Handles terminal UI rendering
   - Processes keyboard input (SPACE, 'q')
   - Sends `HeartMessage::Beat` or `HeartMessage::Stop` to worker

2. **Worker Thread (Heart)**:
   - Runs in background via `std::thread::spawn`
   - Receives messages via `mpsc::channel`
   - Uses `recv_timeout()` to implement multiple timers
   - Publishes updates via `broadcast::channel`

3. **DBus Task** (optional):
   - Async task spawned with `tokio::spawn`
   - Subscribes to `HeartbeatUpdate` events
   - Emits DBus signals to external programs

### Data Flow: User Presses SPACE

```
1. Terminal UI detects SPACE key press
2. Main thread calls session.beat()
3. session.beat() sends HeartMessage::Beat to worker thread
4. Worker thread receives Beat:
   - Updates last_beat timestamp
   - Calculates new elapsed duration
   - Increments AtomicU64 counter
5. Worker broadcasts HeartbeatUpdate::DurationChanged
6. Subscribers receive update:
   - Terminal UI: Updates display
   - DBus Publisher: Emits state_changed signal
7. External programs (taskbar) receive DBus signal
8. Taskbar updates display
```

### Database Persistence

The stopwatch periodically saves to the database using the `LogValueCommand`:

```rust
fn persist_time_frame(&self, start: NaiveTime, end: NaiveTime) {
    1. Get current date
    2. Fetch existing TimeFrameList from repository
    3. Create new TimeFrame(start, end)
    4. Apply UpdateEndValue policy (domain logic handles merging/splitting)
    5. Save to repository if changed
}
```

This means:
- **Data safety**: Progress saved every minute (default)
- **Efficient**: Only writes if data changed
- **Domain logic**: Handles overlapping/adjacent time frames correctly

### Key Design Decisions

1. **Broadcast Channel**: Multiple subscribers can listen independently
2. **Atomic Counter**: Lock-free reads for minimal overhead
3. **Three-tier Timing**: Duration (1s), Flush (1m), Idle (1d) all in one worker
4. **Optional DBus**: Can disable for debugging/minimal systems
5. **Heartbeat-based**: Prevents accidental logging (requires active "beats")

### Performance Characteristics

- **Memory**: ~100 KB (mostly DBus and tokio runtime)
- **CPU**: Negligible (~0.1% on modern systems)
- **I/O**: Database write every flush_interval (default 1 minute)
- **Network**: DBus signal broadcast every 1 second while running

### Testing the DBus Interface

You can test the DBus interface directly using `busctl`:

```bash
# Check if service is running
busctl list | grep lifestats

# Call get_state method
busctl call com.lifestats.Stopwatch \
    /com/lifestats/Stopwatch \
    com.lifestats.Stopwatch1 \
    get_state

# Monitor state_changed signals
busctl monitor com.lifestats.Stopwatch
```

## Troubleshooting

### "No stopwatch active" despite stopwatch running

- Check if DBus is enabled: `DISABLE_DBUS=1` disables the interface
- Verify service is running: `busctl list | grep lifestats`
- Check DBus permissions: Some systems restrict DBus session bus access

### Widget shows stale data

- Increase taskbar refresh rate (set `interval=1` in config)
- Use signal-based approach instead of polling
- Check system load (high load can delay DBus signals)

### Stopwatch stops unexpectedly

- Check idle_timeout setting (default 1 day)
- Verify heartbeats are being sent (press SPACE regularly)
- Check system logs for crashes

## Future Integration Ideas

- **Desktop notifications**: Alert when reaching time goals
- **Statistics overlay**: Show daily/weekly totals
- **Multiple stopwatches**: Track multiple activities simultaneously
- **REST API**: HTTP interface for web-based dashboards
- **Mobile app**: Sync with mobile devices via network API

## Additional Resources

- Client library source: `logger_applications/stopwatch-service/`
- Examples: `logger_applications/stopwatch-service/examples/`
- Core session logic: `core/application/src/use_cases/session_tracking_service.rs`
- DBus implementation: `logger_applications/stopwatch_logger/src/dbus/mod.rs`
