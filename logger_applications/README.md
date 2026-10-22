# Logger Applications

Automatic activity tracking integrations for the LifeStats platform. Each logger captures time and activity data from different software or hardware and sends it to the LifeStats API.

## Quick Reference

| Logger | Tracks | Integration Type | Tech Stack |
|--------|--------|------------------|------------|
| [AnkiLogger](#ankilogger) | Flashcard study time, disruptions, backlog | Anki add-on | Python |
| [Clef Sync](#clef-sync) | Clef active piano practice minutes | SSH snapshot import | Bash |
| [Stopwatch](#stopwatch) | Manual time tracking | CLI application | Rust |
| [MIDI Logger](#midi-logger) | Piano/keyboard practice time | CLI application | Rust |
| [MPV Logger](#mpv-logger) | Video playback time | MPV Lua script | Lua |
| [E-book Reader](#e-book-reader) | Reading time | Web application | TypeScript/Svelte |
| [Web Activity Tracker](#web-activity-time-tracker) | Browsing time per site | Browser extension | TypeScript |
| [Bedtime Watch](#bedtime-watch) | Bedtime (one tap) | Wear OS tile | Kotlin |
| [OpenNutriTracker](#opennutritracker) | Nutrition | Vendored app; Lifestats sync pending | Dart/Flutter |

## Logger Applications

### OpenNutriTracker

[`OpenNutriTracker/`](OpenNutriTracker/) contains a vendored copy of
[simonoppowa/OpenNutriTracker](https://github.com/simonoppowa/OpenNutriTracker),
alongside `LiftLog/`. The app tracks food intake and nutrition on Android and iOS.
Lifestats sync is not implemented yet.

See [`OpenNutriTracker/UPSTREAM.md`](OpenNutriTracker/UPSTREAM.md) for the source
commit and local changes, and
[`OpenNutriTracker/GettingStarted.md`](OpenNutriTracker/GettingStarted.md) for setup.

### Bedtime Watch

**Tracks:** Bedtime (`就寝時間`), filed under the evening's date

**How it works:**
- A Pixel Watch tile logs the current time with one tap
- Saves on the watch first, then uploads with retries, so it works with the phone off
- Reaches the API over the home LAN, with the tailnet URL as a fallback

See [`bedtime_watch/README.md`](bedtime_watch/README.md) for build, install and emulator testing.

### Clef Sync

**Tracks:** Clef's authoritative active piano practice minutes

**How it works:**
- Pulls a versioned rolling daily snapshot from Clef over SSH
- Reuses Clef's input-derived idle cutoff and 03:00 activity-day semantics
- Overwrites an `IntegerAmount` statistic so retries remain idempotent
- Runs hourly from a persistent systemd user timer

See [`clef_sync/README.md`](clef_sync/README.md) for setup and operations.

---

### AnkiLogger

**Tracks:** Flashcard study time, session disruptions, and backlog size per deck

**How it works:**
- Installs as an Anki add-on in the `addons21/` directory
- Hooks into Anki's card review system to detect study activity
- Automatically provisions statistics via the LifeStats API for each deck

**Key features:**
- Per-deck metrics (3 statistics per deck: time, disruptions, backlog)
- 60-second idle timeout prevents inflated time tracking
- Automatic session detection with start/resume counting
- Flushes data every 120 seconds
- Bundles HTTP dependencies to avoid conflicts with other add-ons

**Statistic types:**
- Time Spent: `TimeSpans`
- Disruptions: `IntegerAmount` (counter)
- Backlog: `IntegerAmount` (snapshot of cards due)

**API integration:**
- `POST /integrations/anki` - Provision deck statistics bundle
- `POST /statistics/{id}/log_time_frame/{date}` - Log study time
- `POST /statistics/{id}/log/{date}` - Log disruptions and backlog

---

### Stopwatch

**Tracks:** Any manually-controlled activity time

**How it works:**
- Terminal-based CLI application with real-time display
- Manual start/pause/resume control via SPACE key
- Companion `stopwatch-service` provides DBus interface for desktop widgets

**Key features:**
- Interactive terminal UI with HH:MM:SS display
- 1-minute automatic flush interval
- 1-day idle timeout (essentially no auto-stop)
- Graceful shutdown with 'q' key
- Optional DBus publishing for taskbar widgets (i3blocks, polybar, waybar)

**Statistic types:**
- Time tracking: `TimeSpans` only

**Use cases:**
- Study sessions, deep work, exercise, creative work, household tasks

---

### MIDI Logger

**Tracks:** Piano/keyboard practice time (active playing only)

**How it works:**
- Listens to ALSA MIDI events via the `midir` Rust library
- Terminal UI for selecting MIDI device and target statistic
- Detects activity through MIDI note-on events

**Key features:**
- Automatic activity detection (no manual start/stop)
- 1-minute idle timeout excludes breaks from practice time
- 10-second flush interval
- Heartbeat-based tracking system
- Graceful shutdown with Ctrl+C

**Statistic types:**
- Practice time: `TimeSpans` only

---

### MPV Logger

**Tracks:** Video playback time from specific directories

**How it works:**
- Lua script installed in MPV's scripts directory
- Monitors playback events and filters by target directory
- Sends heartbeat pings to LifeStats during active playback

**Key features:**
- Directory-based filtering (e.g., "Japanese Listening Practice")
- Heartbeat ping every 1 second during playback
- Automatic pause/resume detection
- 1-hour idle timeout, 60-second flush interval
- Uses curl for HTTP requests

**Statistic types:**
- Viewing time: Uses session-based heartbeat API

**Configuration:**
- Statistic ID and target directory are hardcoded in the Lua script
- API endpoint: `http://localhost:3000/sessions`

---

### E-book Reader

**Tracks:** Reading time in an online e-book reader

**How it works:**
- Web application hosted at https://reader.ttsu.app
- Scroll-based activity detection triggers heartbeat pings
- Built with Svelte/TypeScript

**Key features:**
- Dictionary extension support (Yomitan for Japanese learning)
- Character counter optimized for Japanese
- Scroll-based activity detection
- Local statistics tracking in browser

**Note:** Created primarily for Japanese language learners. Other languages may work with limited functionality.

---

### Web Activity Time Tracker

**Tracks:** Total browsing time with per-site breakdown

**How it works:**
- Browser extension for Chrome/Firefox
- Tracks active tab time per domain
- Logs data to LifeStats every 20 seconds

**Key features:**
- Per-website domain tracking
- Site blocking after daily limit expiration
- Whitelist support for filtering tracked sites
- Built-in dashboard and statistics UI
- Automatic logging to LifeStats

**Statistic types:**
- Online time: `IntegerAmount` (logs seconds with sum policy)

**API integration:**
- `POST /statistics/16/log/{date}` - Logs browsing time every 20 seconds (hardcoded to statistic ID 16)

## Common Patterns

### Heartbeat-Based Tracking
Most live loggers use a heartbeat/ping pattern to track active time:
- **MIDI Logger:** MIDI note-on events trigger heartbeats
- **MPV Logger:** Periodic pings every second during playback
- **E-book Reader:** Scroll events trigger activity pings
- **Stopwatch:** Manual control with automatic flushes

Clef Sync deliberately imports Clef's finished daily snapshot instead of
creating a second heartbeat clock.

### Idle Detection
Prevents inflated time tracking during inactivity:
- **AnkiLogger:** 60 seconds
- **MIDI Logger:** 60 seconds
- **MPV Logger:** 3600 seconds (1 hour)
- **Stopwatch:** 1 day (essentially disabled)

### Statistic Types
- **TimeSpans:** AnkiLogger, MIDI Logger, Stopwatch, MPV Logger
- **IntegerAmount:** AnkiLogger (disruptions, backlog), Web Activity Tracker

## Getting Started

Each logger has its own README with installation and configuration instructions. See the individual directories for details:

- `AnkiLogger/README.md`
- `clef_sync/README.md`
- `stopwatch/stopwatch_logger/README.md`
- `midi_logger/README.md`
- `mpv_logger/README.md`
- `ebook-reader/README.md`
- `web-activity-time-tracker/README.md`

### Workspace Status

**Note:** According to the project's `CLAUDE.md`, most logger applications are excluded from the main Cargo workspace and may use outdated APIs. Active workspace members include:
- `midi_logger`
- `stopwatch/stopwatch_logger`

Other loggers may require updates to work with the latest LifeStats API.

## Architecture

All loggers follow a common pattern:
1. **Activity Detection** - Detect when the user is actively engaged
2. **Heartbeat Tracking** - Maintain a heartbeat to track duration
3. **Idle Detection** - Stop tracking after period of inactivity
4. **API Integration** - Send time frames or log values to LifeStats HTTP API

Most loggers use the LifeStats API endpoints:
- `POST /statistics/{id}/log_time_frame/{date}` - Log time spans
- `POST /statistics/{id}/log/{date}` - Log integer amounts
- `POST /sessions/{id}/start` - Start session-based tracking
- `POST /sessions/{id}/beat` - Heartbeat ping
- `POST /sessions/{id}/stop` - End session

See `core/infrastructure/web_api/README.md` for full API documentation.
