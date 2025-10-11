# MIDI Logger

A MIDI-based time tracking application for logging piano practice sessions (or any MIDI instrument) to the LifeStats platform.

## Overview

The MIDI Logger listens to MIDI input from your digital piano or MIDI keyboard and automatically logs practice time to a statistic in your LifeStats database. It's designed to track *actual* playing time, not just the time you have your instrument turned on.

## Key Features

- **Automatic Activity Detection**: Detects when you're actively playing by listening for MIDI note-on events
- **Smart Idle Timeout**: Automatically pauses tracking after **1 minute of inactivity**, so breaks and distractions don't inflate your practice time
- **Interactive Setup**: Simple terminal UI to select your MIDI device and target statistic
- **Real-time Logging**: Flushes data every 10 seconds to prevent data loss
- **Graceful Shutdown**: Press `Ctrl+C` to stop logging and ensure all data is saved

## How It Works

The logger uses a **heartbeat-based tracking system**:

1. Every MIDI note-on event sends a "heartbeat" to the tracking session
2. As long as heartbeats occur within 1 minute of each other, the session remains active
3. If no notes are played for 1 minute, the session automatically stops
4. When you start playing again (after the timeout), a new session begins
5. All sessions are logged as time intervals to your chosen statistic

This ensures that only *active playing time* is recorded, not time spent adjusting settings, taking breaks, or being distracted.

## Requirements

- **MIDI Device**: A digital piano, MIDI keyboard, or any MIDI input device
- **Rust**: Version 1.70 or later
- **ALSA** (Linux): ALSA development libraries for MIDI support
  ```bash
  # Ubuntu/Debian
  sudo apt-get install libasound2-dev

  # Fedora
  sudo dnf install alsa-lib-devel
  ```

## Building

From the repository root:

```bash
cargo build -p midi_logger --release
```

The binary will be located at `target/release/midi_logger`.

## Usage

1. **Connect your MIDI device** to your computer before starting the logger

2. **Run the logger**:
   ```bash
   cargo run -p midi_logger --release
   ```

   Or if you've built it:
   ```bash
   ./target/release/midi_logger
   ```

3. **Select your MIDI port**:
   - If only one MIDI device is connected, it will be selected automatically
   - If multiple devices are available, you'll see a numbered list—enter the number of your piano/keyboard

4. **Select a statistic to log to**:
   - The logger will display all available "TimeSpans" statistics in your database
   - Enter the ID number of the statistic where you want to log your practice time
   - Type `q` to quit if needed

5. **Start playing**:
   - Once connected, you'll see a session header with your selected statistic and MIDI port
   - Play your instrument normally—the logger tracks all note-on events
   - The logger runs silently in the background

6. **Stop the session**:
   - Press `Ctrl+C` when you're done practicing
   - The logger will save all tracked time and shut down gracefully

## Configuration

Current settings (hardcoded in `main.rs:62-65`):

- **Idle timeout**: 1 minute (60 seconds)
- **Flush interval**: 10 seconds

To modify these values, edit the `HeartbeatConfig` in `src/main.rs`:

```rust
HeartbeatConfig {
    idle_timeout: chrono::Duration::minutes(1),  // Change this
    flush_interval: chrono::Duration::seconds(10), // Change this
}
```

## Troubleshooting

### No MIDI ports found

- Ensure your MIDI device is connected and powered on
- On Linux, check that ALSA recognizes your device:
  ```bash
  aconnect -l
  ```
- Verify your user has permissions to access MIDI devices

### No TimeSpans statistics available

- You need at least one TimeSpans-type statistic in your LifeStats database
- Create one using the LifeStats CLI or web interface before running the logger

### Logger doesn't detect notes

- Enable debug logging to see raw MIDI events:
  ```bash
  RUST_LOG=debug cargo run -p midi_logger
  ```
- Check that your device is sending MIDI note-on messages (status byte `0x9n`)
- Some devices may send MIDI over a different channel—verify your device configuration

### Session ends unexpectedly

- If your playing has gaps longer than 1 minute, the session will stop automatically
- To change this behavior, increase the `idle_timeout` duration in the configuration

## Architecture

This is a **Driving Adapter** in the LifeStats hexagonal architecture. It translates MIDI hardware events into application layer use cases:

- `MidiAdapter`: Hardware abstraction layer for MIDI input
- `HeartbeatSession`: Application layer session tracking
- `TerminalUI`: User interaction for setup

The logger depends on:
- `lifestats-bootstrap`: Service composition
- `lifestats-application`: Use case handlers
- `lifestats-core`: Domain types
