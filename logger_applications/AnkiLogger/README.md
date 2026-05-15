# AnkiLogger

An Anki addon that automatically tracks your study sessions and logs them to the LifeStats personal observability platform.

## Overview

AnkiLogger integrates with Anki to automatically track:
- **Time spent studying** - Continuous tracking of active study time with automatic idle detection
- **Study disruptions** - Counts how many times you start/resume a study session
- **Backlog size** - Tracks the total number of cards due (new + learning + review)

All metrics are logged per deck and sent to the LifeStats web API for visualization and analysis.

## Features

### Automatic Session Tracking
- Automatically starts tracking when you begin reviewing cards
- Detects user activity through card interactions (show question, show answer, card answered)
- Stops tracking after 60 seconds of inactivity (configurable)
- Flushes time frames every 120 seconds (configurable)

### Per-Deck Metrics
Each deck gets three statistics automatically provisioned:
- **Time Spent** - TimeSpans statistic tracking study session durations
- **Disruptions** - Counter for session starts (indicates focus interruptions)
- **Backlog** - Snapshot of total cards due; logged on Anki startup for already-provisioned decks and refreshed during study sessions

### Automatic Provisioning
- First time you study a deck, AnkiLogger contacts the LifeStats API to provision statistics
- Provisions are cached locally in Anki's config
- Subsequent sessions use cached statistic IDs (no API calls needed for setup)
- Startup backlog snapshots only use cached statistic IDs; decks that have not been provisioned are skipped

### Clean Architecture
Following the **Ports & Adapters (Hexagonal) Architecture** pattern:
- `SessionTracker` - Core business logic for session management
- `ApiClient` - Adapter for HTTP communication with LifeStats API
- `ConfigManager` - Adapter for Anki configuration storage
- `hooks.py` - Driving adapter that connects Anki events to the tracker

## Installation

### Prerequisites
- Anki 2.1.60+ (uses modern addon API)
- LifeStats web API running (default: `http://localhost:3000`)

### Setup
1. Copy the `AnkiLogger` directory to your Anki addons folder:
   ```bash
   # Linux
   ~/.local/share/Anki2/addons21/AnkiLogger/

   # macOS
   ~/Library/Application Support/Anki2/addons21/AnkiLogger/

   # Windows
   %APPDATA%\Anki2\addons21\AnkiLogger\
   ```

2. Restart Anki

3. The addon will initialize on startup and begin tracking when you start reviewing

## Configuration

### API Base URL
Edit `config.json` to set your LifeStats API endpoint:
```json
{
    "api_base_url": "http://localhost:3000"
}
```

### Deck-Specific Configuration
The addon automatically provisions and caches statistic IDs in `meta.json`. You normally don't need to edit this file manually.

Example cached configuration:
```json
{
  "config": {
    "deck_configs": {
      "1661059140274": {
        "time_spent_id": 41,
        "disruptions_id": 42,
        "backlog_id": 43
      }
    }
  }
}
```

## How It Works

### Session Lifecycle

0. **Anki Startup**
   - Addon initializes and loads cached deck statistic bundles
   - For each already-provisioned deck, logs the current backlog count
   - Unprovisioned decks are skipped to avoid creating unwanted LifeStats statistics

1. **Session Start**
   - User begins reviewing cards (triggers `state_did_change` or card interaction hooks)
   - `SessionTracker.ping()` is called
   - If not already running, provisions/retrieves deck statistics
   - Logs initial disruption event (+1 to disruptions counter)
   - Logs current backlog count (new + learning + review)
   - Starts idle timeout and flush interval timers

2. **Active Session**
   - Each card interaction calls `ping()` to signal user activity
   - Every 120 seconds (flush interval), logs accumulated time frame
   - Idle timer resets on each `ping()`

3. **Session End**
   - After 60 seconds of no activity, session times out
   - Or when user explicitly exits reviewer
   - Logs final time frame
   - Resets session state

### API Integration

The addon communicates with three LifeStats API endpoints:

**Provision Bundle**
```
POST /integrations/anki
{
  "deck_id": "1661059140274",
  "deck_name": "日本語"
}
→ Returns: { time_spent_id, disruptions_id, backlog_id }
```

**Log Time Frame**
```
POST /statistics/{statistic_id}/log_time_frame/{date}
{
  "start_time": "14:30:15",
  "end_time": "14:32:45",
  "update_policy": "updateendvalue"
}
```

**Log Statistic Value**
```
POST /statistics/{statistic_id}/log/{date}
{
  "value": "1",
  "update_policy": "sum"  # or "overwrite"
}
```

### Idle Detection
- **Idle timeout**: 60 seconds (default)
- User is considered idle if no card interactions occur within the timeout window
- On idle, session stops and final time frame is logged
- Next card interaction starts a new session (logs new disruption)

### Time Frame Flushing
- **Flush interval**: 120 seconds (default)
- While active, time frames are periodically flushed to the API
- This ensures progress is saved even during long study sessions
- Uses `updateendvalue` policy to extend the current time span

## Dependencies

### Bundled (Vendored)
The addon bundles its HTTP dependencies to avoid conflicts with other addons:
- `requests` 2.32.5
- `urllib3` 2.5.0
- `charset-normalizer` (via requests)
- `idna` (via requests)

These are located in the `vendor/` directory and automatically added to Python's path.

### Anki APIs Used
- `aqt.gui_hooks` - Event system for card reviews
- `aqt.mw.col.decks` - Deck information access
- `aqt.mw.col.sched` - Scheduler data for backlog counting
- `aqt.utils.tooltip` - User notifications

## Development

### Logging
Logs are written to `anki_logger.log` in the addon directory:
```bash
tail -f ~/.local/share/Anki2/addons21/AnkiLogger/anki_logger.log
```

Log levels:
- `INFO` - Session lifecycle events, API calls
- `DEBUG` - Hook triggers, ping events, internal state
- `ERROR` - API failures, exceptions

### Testing Changes
1. Edit addon files
2. Restart Anki (Tools → Add-ons → reload is insufficient)
3. Begin reviewing to trigger session
4. Check `anki_logger.log` for errors

### Key Components

**`session_tracker.py:SessionTracker`**
- Core application logic
- Manages session state and timing
- Orchestrates provisioning, logging, and idle detection
- Independent of Anki-specific code (easy to test)

**`api_client.py:ApiClient`**
- Stateless HTTP adapter
- Handles all web API communication
- Provides both sync and async logging methods
- Async methods use ThreadPoolExecutor to keep UI responsive

**`hooks.py`**
- Registers Anki event handlers
- Translates Anki events into `SessionTracker` calls
- Minimal logic - just wiring

**`config_manager.py:ConfigManager`**
- Manages deck bundle cache in Anki config
- Simple key-value interface for statistic bundles
- Persistence adapter

## Troubleshooting

### "Failed to configure deck" tooltip
- **Cause**: Cannot reach LifeStats API
- **Check**: Is the web API running at the configured base URL?
- **Check**: Review `anki_logger.log` for connection errors

### Statistics not appearing in LifeStats
- **Check**: Verify deck is provisioned (check `meta.json` for deck_configs entry)
- **Check**: Look for HTTP errors in `anki_logger.log`
- **Try**: Delete deck entry from `meta.json` to force re-provisioning

### Session not starting
- **Check**: Make sure you're actually reviewing cards (not browsing or editing)
- **Check**: Look for "ping() called" messages in log
- **Check**: Verify no exceptions during addon initialization

### Idle timeout too short/long
Currently hardcoded to 60 seconds. To change:
1. Edit `__init__.py` → `AddonManager.__init__`
2. Modify `idle_timeout_seconds` parameter
3. Restart Anki

## Architecture Diagram

```
┌─────────────────────────────────────────────────┐
│                  Anki Hooks                     │
│  (state_did_change, reviewer_did_answer_card)  │
└────────────────────┬────────────────────────────┘
                     │
                     ▼
            ┌─────────────────┐
            │  hooks.py       │
            │  (Event Router) │
            └────────┬────────┘
                     │
                     ▼
            ┌─────────────────────┐
            │  SessionTracker     │
            │  (Core Logic)       │
            │  - Session state    │
            │  - Idle detection   │
            │  - Timer management │
            └──────┬──────┬───────┘
                   │      │
          ┌────────┘      └────────┐
          ▼                        ▼
   ┌─────────────┐         ┌──────────────┐
   │ ApiClient   │         │ ConfigManager│
   │ (HTTP)      │         │ (Cache)      │
   └──────┬──────┘         └──────┬───────┘
          │                       │
          ▼                       ▼
   ┌─────────────┐         ┌──────────────┐
   │ LifeStats   │         │ Anki Config  │
   │ Web API     │         │ (meta.json)  │
   └─────────────┘         └──────────────┘
```

## License

Part of the LifeStats personal observability platform.
