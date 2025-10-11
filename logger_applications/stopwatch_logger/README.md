# Stopwatch Logger

A manual time tracking application for logging activities to the LifeStats platform with a simple stopwatch interface.

## Overview

The Stopwatch Logger provides a terminal-based stopwatch for manually tracking time spent on any activity. It's perfect for tracking study sessions, work tasks, exercise, or any activity where you want manual control over when tracking starts and stops.

## Key Features

- **Manual Control**: Start, pause, and resume tracking with a simple keypress
- **Real-time Display**: Shows elapsed time in HH:MM:SS format
- **Interactive Setup**: Terminal UI to select which statistic to log to
- **Automatic Data Persistence**: Flushes data every 1 minute to prevent data loss
- **Graceful Shutdown**: Press `q` to quit and ensure all data is saved
- **Extended Idle Tolerance**: 1-day idle timeout allows for long breaks without losing your session

## How It Works

The logger uses a **heartbeat-based tracking system**:

1. When you start the stopwatch, it begins tracking time immediately
2. Pressing SPACE pauses the session (stops tracking)
3. Pressing SPACE again resumes tracking from where you left off
4. Time is automatically saved every 1 minute while running
5. Pressing `q` stops the session and saves all data before exiting

Unlike automatic trackers, this gives you complete manual control—useful for activities where you want to consciously decide when you're "on the clock."

## Requirements

- **Rust**: Version 1.70 or later
- **LifeStats database**: Must have at least one TimeSpans statistic configured

## Building

From the repository root:

```bash
cargo build -p StopwatchLogger --release
```

The binary will be located at `target/release/StopwatchLogger`.

## Usage

1. **Run the logger**:
   ```bash
   cargo run -p StopwatchLogger --release
   ```

   Or if you've built it:
   ```bash
   ./target/release/StopwatchLogger
   ```

2. **Select a statistic to log to**:
   - The logger displays all available "TimeSpans" statistics from your database
   - Enter the ID number of the statistic you want to track time for
   - Type `q` to quit if needed

3. **Use the stopwatch**:
   - Once started, you'll see:
     ```
     Stopwatch for 'Your Activity'. Press SPACE to pause/resume, 'q' to quit.
     Elapsed time: 00:05:23
     Status: Running
     ```
   - **SPACE**: Pause/resume the stopwatch
   - **q**: Quit and save all tracked time

4. **Track your activity**:
   - The stopwatch starts automatically after selection
   - Pause when you take breaks
   - Resume when you continue working
   - All time is accumulated in the same session

5. **Finish tracking**:
   - Press `q` when done—the logger saves all data and exits gracefully

## Configuration

Current settings (hardcoded in `main.rs:30-33`):

- **Idle timeout**: 1 day (very long, essentially no automatic stopping)
- **Flush interval**: 1 minute

To modify these values, edit the `HeartbeatConfig` in `src/main.rs`:

```rust
HeartbeatConfig {
    idle_timeout: chrono::Duration::days(1),      // Change this
    flush_interval: chrono::Duration::minutes(1),  // Change this
}
```

## Troubleshooting

### No TimeSpans statistics available

- You need at least one TimeSpans-type statistic in your LifeStats database
- Create one using the LifeStats CLI or web interface before running the logger

### Invalid ID error

- Ensure you're entering a valid ID number from the displayed list
- Only numeric input is accepted (non-digit characters are ignored)
- Press `q` to quit the selection dialog

### Data not saving

- The logger flushes data every 1 minute automatically
- Always quit using `q` rather than force-killing the process to ensure data is saved
- Check the LifeStats database connection if issues persist

## Use Cases

The Stopwatch Logger is ideal for:

- **Study sessions**: Track focused study time with breaks excluded
- **Deep work**: Monitor time spent on specific tasks or projects
- **Exercise**: Log workout duration (pause during rest periods)
- **Creative work**: Track time spent writing, composing, or designing
- **Household tasks**: Time how long chores or projects actually take
- **Learning**: Monitor time practicing a skill or language

## Architecture

This is a **Driving Adapter** in the LifeStats hexagonal architecture. It provides a manual user interface for time tracking:

- `TerminalInterface`: Terminal-based UI for stopwatch display and control
- `HeartbeatSession`: Application layer session tracking with pause/resume
- `UserInterface` trait: Abstraction allowing alternative UI implementations

The logger depends on:
- `lifestats-bootstrap`: Service composition and dependency injection
- `lifestats-application`: Use case handlers and session management
- `crossterm`: Cross-platform terminal manipulation

## Comparison with Other Loggers

| Logger | Trigger | Use Case |
|--------|---------|----------|
| **Stopwatch** | Manual (keypress) | Activities where you control start/stop |
| **MIDI Logger** | Automatic (MIDI events) | Instrument practice with automatic detection |
| **MPV Logger** | Automatic (media events) | Video watching time |
| **Fitbit Logger** | Scheduled sync | Health metrics from wearables |

Choose the Stopwatch Logger when you want **manual control** and **conscious awareness** of time tracking.
