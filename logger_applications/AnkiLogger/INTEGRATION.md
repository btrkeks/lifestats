# Anki Bundle Integration Guide

## Quick Start

Anki bundles are sets of related statistics that are automatically provisioned together when you start tracking a new deck. This guide will walk you through adding a new metric to the existing Anki bundle.

**Prerequisites:**
- LifeStats API server running (`cargo run -p lifestats-web-api`)
- Anki installed with AnkiLogger add-on
- Basic familiarity with Rust and Python

**Related Documentation:**
- Main architecture: `../../CLAUDE.md`
- Detailed architecture diagrams: `../../docs/ARCHITECTURE.md`
- Statistic types reference: `../../docs/statistics.md`

## What is an Anki Bundle?

An **Anki bundle** is a predefined set of four related statistics that are automatically created and managed together for each Anki deck you study. The bundle pattern provides:

- **Automatic provisioning**: Statistics are created on-the-fly when you first study a deck
- **Idempotency**: Calling the provision API multiple times returns the same statistic IDs
- **Lifecycle management**: All metrics for a deck are grouped together via tags
- **Atomic operations**: Bundle creation is all-or-nothing

### Current Bundle Metrics

Each Anki deck gets these four metrics:

| Metric | Type | Purpose | When Logged |
|--------|------|---------|-------------|
| **Time Spent** | TimeSpans | Tracks cumulative duration of study sessions | Every 120 seconds during session |
| **Disruptions** | IntegerAmount | Counts how many times you start/resume studying | Each session start |
| **Backlog** | IntegerAmount | Snapshot of total cards due (new + learning + review) | Session start and end |
| **Finished Time** | PointInTime | Records timestamp when all cards in deck were completed | When deck becomes empty |

### Why Bundles?

Bundles solve several problems:
1. **Consistency**: Every deck gets the same set of metrics automatically
2. **Discovery**: No manual configuration needed - just start studying
3. **Versioning**: Tags allow upgrading bundle structure over time
4. **Identity**: Tag-based lookup ensures we reuse existing statistics

## Architecture Overview

### Data Flow

```
┌─────────────────┐
│   Anki Events   │ (user studies cards)
└────────┬────────┘
         │
         ▼
    ┌──────────┐
    │ hooks.py │ (event router - minimal logic)
    └────┬─────┘
         │
         ▼
┌──────────────────┐
│ session_tracker  │ (core business logic)
│  - Session state │
│  - Idle detect   │
│  - Timer mgmt    │
└────┬──────┬──────┘
     │      │
     │      └──────────────┐
     │                     │
     ▼                     ▼
┌────────────┐    ┌────────────────┐
│ api_client │    │ config_manager │
│  (HTTP)    │    │  (cache)       │
└─────┬──────┘    └────────┬───────┘
      │                    │
      ▼                    ▼
┌─────────────┐    ┌──────────────┐
│ Web API     │    │  meta.json   │
│ (port 3000) │    │  (disk)      │
└─────────────┘    └──────────────┘
```

### Component Responsibilities

**Python Side (AnkiLogger):**

- **`hooks.py`** - Event router
  - Registers Anki event handlers using `aqt.gui_hooks`
  - Translates Anki events to SessionTracker calls
  - Pure wiring, no business logic

- **`session_tracker.py`** - Core business logic
  - Manages session state (running/stopped, start_time, last_ping)
  - Handles idle detection (60-second timeout)
  - Orchestrates periodic flushing (every 120 seconds)
  - Decides when to log each metric
  - Pure logic (no Anki API calls)

- **`api_client.py`** - HTTP adapter
  - Stateless HTTP client to LifeStats API
  - Provides sync and async methods (uses ThreadPoolExecutor)
  - Defines `StatisticBundle` dataclass

- **`config_manager.py`** - Persistence adapter
  - Reads/writes to Anki's addon config storage
  - Caches provisioned statistic IDs locally
  - Simple key-value interface

- **`anki_utils.py`** - Anki API interface
  - Queries Anki's scheduler and deck tree
  - Pure query functions (no state)

**Rust Side (LifeStats API):**

- **`AnkiBundleFactory`** - Bundle definition
  - Implements `BundleFactory` trait
  - Defines the 4 metrics and their configuration
  - Creates statistics with appropriate tags

- **`IntegrationService`** - Provision orchestration
  - Dispatches to correct factory based on source
  - Handles idempotency via tag lookup
  - Returns response with statistic IDs

- **`POST /integrations/{source}`** - HTTP endpoint
  - Accepts provision requests
  - Returns bundle of statistic IDs as JSON

### Hexagonal Architecture

LifeStats uses **Ports & Adapters (Hexagonal) Architecture**:

```
┌──────────────────────────────────────┐
│         Domain Layer (Core)          │
│  - Pure business logic               │
│  - Statistic types                   │
│  - Port trait definitions            │
│  - ZERO external dependencies        │
└──────────────┬───────────────────────┘
               │ (dependencies point inward)
               │
┌──────────────▼───────────────────────┐
│      Application Layer               │
│  - Use case orchestration (CQRS)     │
│  - Command/Query handlers            │
│  - BundleFactory trait & impls       │
│  - IntegrationService                │
└──────────────┬───────────────────────┘
               │
┌──────────────▼───────────────────────┐
│    Infrastructure Layer              │
│  - HTTP API (Axum)                   │
│  - Database (Diesel + SQLite)        │
│  - Event publishing                  │
└──────────────────────────────────────┘
```

**Key principle**: Dependencies always point inward toward the domain. The domain knows nothing about databases, HTTP, or external services.

## Step-by-Step: Adding a New Metric

### Step 1: Decide on Metric Details

Before coding, plan your metric using this template:

| Aspect | Your Decision |
|--------|---------------|
| **Metric Name** | e.g., `cards_studied` |
| **Statistic Type** | `TimeSpans` / `IntegerAmount` / `Habit` / `PointInTime` |
| **When to Log** | e.g., "after each card" / "on session end" / "periodically" |
| **Update Policy** | `sum` / `overwrite` / `ignore` / `append` / `updateendvalue` |
| **Data Source** | e.g., `anki_utils.get_cards_studied_today(deck_id)` |
| **Purpose** | Brief description of what insight this provides |

**Example:**
- **Name**: `cards_studied`
- **Type**: `IntegerAmount`
- **When**: On session end
- **Policy**: `sum` (accumulate across multiple sessions)
- **Source**: Query Anki's revlog for today's reviews
- **Purpose**: Track daily card review count per deck

### Step 2: Modify the Rust Backend

**File:** `core/application/src/use_cases/integration/anki_bundle_factory.rs`

**Change 1:** Update the `ANKI_BUNDLE_METRICS` constant

```rust
// OLD (4 metrics)
const ANKI_BUNDLE_METRICS: [&str; 4] = [
    "time_spent",
    "disruptions",
    "backlog",
    "finished_time",
];

// NEW (5 metrics)
const ANKI_BUNDLE_METRICS: [&str; 5] = [
    "time_spent",
    "disruptions",
    "backlog",
    "finished_time",
    "cards_studied",  // ADD YOUR METRIC HERE
];
```

**Change 2:** Add metric creation logic in the `provision()` method

Find the section where other metrics are created and add your new one:

```rust
impl BundleFactory for AnkiBundleFactory {
    fn provision(&self, context: ProvisionContext) -> Result<Vec<ProvisionedStatistic>> {
        let deck_name = context.get("deck_name")
            .ok_or_else(|| anyhow!("deck_name is required"))?;

        let mut bundle = Vec::new();

        // ... existing metrics (time_spent, disruptions, etc.) ...

        // ADD YOUR NEW METRIC HERE:
        let cards_studied_stat = IntegerAmountStatistic::new(
            IntegerAmountMetadata {
                name: format!("{} - Cards Studied", deck_name),
                description: Some(format!(
                    "Number of cards studied in {}",
                    deck_name
                )),
                is_higher_better: Some(true),
                update_policy: Some(AmountUpdatePolicy::Sum),
            }
        )
        .with_tag("source", "anki")
        .with_tag("deck_name", deck_name)
        .with_tag("metric", "cards_studied");

        bundle.push(ProvisionedStatistic {
            key: "cards_studied".to_string(),
            statistic: AnyStatistic::IntegerAmount(cards_studied_stat),
        });

        Ok(bundle)
    }
}
```

**Important details:**

- **Tags are critical**: The combination of `source`, `deck_name`, and `metric` tags makes each statistic unique and enables idempotency
- **Choose the right type**: Use `IntegerAmountStatistic`, `TimeSpansStatistic`, `HabitStatistic`, or `PointInTimeStatistic`
- **Update policy matters**: See section 5 below for detailed explanation
- **Key naming**: The `key` field (e.g., `"cards_studied"`) becomes `"cards_studied_id"` in the API response

### Step 3: Update Python Data Structure

**File:** `logger_applications/AnkiLogger/api_client.py`

Add your new field to the `StatisticBundle` dataclass:

```python
@dataclass
class StatisticBundle:
    """Bundle of statistic IDs for a deck."""
    time_spent_id: int
    disruptions_id: int
    backlog_id: int
    finished_time_id: int
    cards_studied_id: int  # ADD YOUR NEW FIELD HERE
```

**Important:** The field name must match: `{key}_id` where `key` is what you used in Rust.

### Step 4: Add Logging Logic

**File:** `logger_applications/AnkiLogger/session_tracker.py`

Choose where to log your metric based on when you want to capture it:

**Option A: Log during active session (periodic or per-card)**

Add to the `ping()` method:

```python
def ping(self) -> None:
    """Called whenever user interacts with a card."""

    # ... existing provisioning and session start logic ...

    # ADD YOUR METRIC LOGGING HERE:
    if self._session_running and self._active_bundle:
        try:
            cards_count = anki_utils.get_cards_studied_today(self._current_deck_id)
            self._api_client.log_statistic_value_async(
                statistic_id=self._active_bundle.cards_studied_id,
                date=self._get_today(),
                value=str(cards_count),
                update_policy="overwrite"  # Current snapshot
            )
        except Exception as e:
            self._logger.error(f"Failed to log cards studied: {e}")
```

**Option B: Log at session end (summary metrics)**

Add to the `stop_running()` method:

```python
def stop_running(self) -> None:
    """Stop the current session and log final values."""

    if not self._session_running or not self._active_bundle:
        return

    # ... existing flush logic ...

    # ADD YOUR METRIC LOGGING HERE:
    try:
        cards_count = anki_utils.get_cards_studied_today(self._current_deck_id)
        self._api_client.log_statistic_value_async(
            statistic_id=self._active_bundle.cards_studied_id,
            date=self._get_today(),
            value=str(cards_count),
            update_policy="sum"  # Accumulate if called multiple times
        )
    except Exception as e:
        self._logger.error(f"Failed to log cards studied: {e}")

    # ... rest of stop logic ...
```

**Option C: Log at session start (initial state)**

Add to the `_provision_and_start_session()` method where disruptions and backlog are logged.

### Step 5: Add Data Source (if needed)

**File:** `logger_applications/AnkiLogger/anki_utils.py`

If you need to query new data from Anki, add a helper function:

```python
def get_cards_studied_today(deck_id: int) -> int:
    """
    Get count of cards studied today for a specific deck.

    Args:
        deck_id: The Anki deck ID

    Returns:
        Number of cards reviewed today
    """
    mw = aqt.mw
    if not mw or not mw.col:
        return 0

    # Get today's cutoff timestamp
    today_cutoff = mw.col.sched.day_cutoff
    yesterday_cutoff = today_cutoff - 86400  # 24 hours ago

    # Query the revlog (review log) for cards from this deck studied today
    # Note: Anki stores review history with timestamps
    reviews = mw.col.db.all(
        """
        SELECT COUNT(DISTINCT r.cid)
        FROM revlog r
        JOIN cards c ON c.id = r.cid
        WHERE c.did = ?
        AND r.id > ?
        """,
        deck_id,
        yesterday_cutoff * 1000  # Convert to milliseconds
    )

    return reviews[0][0] if reviews else 0
```

**Tips for querying Anki:**
- Use `aqt.mw.col.db` for database queries
- Check `aqt.mw.col.sched` for scheduler data
- See existing functions in `anki_utils.py` for patterns
- Anki's database schema: search for "Anki database schema" documentation

### Step 6: Clear Cache and Test

**Critical:** Python caches the bundle structure. After changes, you must clear the cache.

**Clear cache:**
1. Open Anki
2. Tools → Add-ons → AnkiLogger → View Files
3. Open `meta.json`
4. Delete the entire `"deck_configs"` object (or just your test deck)
5. Save and close

**Rebuild and test:**

```bash
# Terminal 1: Rebuild and start API server
cd /home/daniel/.local/share/src/lifestats
cargo build -p lifestats-web-api
cargo run -p lifestats-web-api

# Terminal 2: Watch logs
# (API server logs will show provisioning requests)

# Restart Anki to reload the add-on
```

**Test flow:**
1. Start Anki and open your test deck
2. Begin reviewing cards
3. Check API server logs for:
   - `POST /integrations/anki` request
   - Response should include `"cards_studied_id": <some_number>`
4. Study a few cards
5. Verify logging via API:
   ```bash
   # List all statistics to find your new one
   curl http://localhost:3000/statistics

   # Get specific statistic data
   curl http://localhost:3000/statistics/<id>
   ```

### Step 7: Verify in Database

Optional but helpful for debugging:

```bash
# Connect to your database
sqlite3 ~/.local/share/lifestats/lifestats.db

# List all statistics with "cards_studied" in the name
SELECT id, name, type FROM statistics WHERE name LIKE '%Cards Studied%';

# Check logged values for a specific statistic
SELECT * FROM logged_values WHERE statistic_id = <your_id>;
```

## Understanding Update Policies

Update policies control how new values interact with existing values. This is critical to get right.

### Policy Reference

| Policy | Behavior | Example Use Case | Formula |
|--------|----------|------------------|---------|
| **`sum`** | Add to existing | Counters, totals | `new = old + value` |
| **`overwrite`** | Replace existing | Current state snapshots | `new = value` |
| **`ignore`** | Only log if empty | One-time events | `new = old \|\| value` |
| **`append`** | Add to list | Time frames (TimeSpans only) | `new = old.append(value)` |
| **`updateendvalue`** | Extend last item | Continuous time tracking | `new[-1].end = value.end` |

### Common Patterns

**Pattern 1: Cumulative counter**
```python
# Use case: Count total disruptions across all sessions
update_policy="sum"
# Day 1, session 1: log 1 → total: 1
# Day 1, session 2: log 1 → total: 2
# Day 1, session 3: log 1 → total: 3
```

**Pattern 2: Current state snapshot**
```python
# Use case: Track current backlog size
update_policy="overwrite"
# Session start: log 50 → stored: 50
# Session end: log 45 → stored: 45 (replaces 50)
```

**Pattern 3: One-time event**
```python
# Use case: Record when deck was first completed
update_policy="ignore"
# First completion at 18:30 → stored: "18:30:00"
# Second completion at 19:45 → stored: "18:30:00" (unchanged)
```

**Pattern 4: Session time tracking**
```python
# Use case: Track multiple study sessions in one day
update_policy="append"  # For TimeSpans
# Session 1: 14:00-14:30 → stored: [(14:00, 14:30)]
# Session 2: 16:00-16:45 → stored: [(14:00, 14:30), (16:00, 16:45)]
```

**Pattern 5: Continuous session extension**
```python
# Use case: Extend current session as user studies
update_policy="updateendvalue"  # For TimeSpans
# Flush 1: 14:00-14:02 → stored: [(14:00, 14:02)]
# Flush 2: 14:00-14:04 → stored: [(14:00, 14:04)]  # Extended end time
```

### Choosing the Right Policy

Ask yourself:
- **Is this cumulative?** → `sum`
- **Is this a snapshot of current state?** → `overwrite`
- **Should this only be recorded once?** → `ignore`
- **Are there multiple discrete events per day?** → `append` (TimeSpans only)
- **Is this a continuous session?** → `updateendvalue` (TimeSpans only)

## Key Files Reference

### Rust Backend

**Bundle Definition & Provisioning:**
- `core/application/src/use_cases/integration/anki_bundle_factory.rs` - Where you define bundle metrics
- `core/application/src/use_cases/integration/service.rs` - Provision orchestration
- `core/application/src/use_cases/integration/bundle_factory.rs` - `BundleFactory` trait

**API Endpoints:**
- `core/infrastructure/web_api/src/routes/integrations.rs` - `POST /integrations/{source}` endpoint
- `core/infrastructure/web_api/src/routes/statistics.rs` - Logging endpoints

**Domain Types:**
- `core/core/src/statistic/types/logable_statistics/` - Logable statistic types
- `core/core/src/statistic/types/derived_statistics/` - Derived statistic types
- `core/core/src/statistic/types/mod.rs` - `AnyStatistic` enum

### Python Add-on

**Core Logic:**
- `logger_applications/AnkiLogger/hooks.py` - Anki event bindings (MODIFY: add new events if needed)
- `logger_applications/AnkiLogger/session_tracker.py` - Session state & logging (MODIFY: add logging calls)
- `logger_applications/AnkiLogger/api_client.py` - HTTP client & bundle dataclass (MODIFY: add field)
- `logger_applications/AnkiLogger/anki_utils.py` - Anki queries (MODIFY: add data source)

**Configuration:**
- `logger_applications/AnkiLogger/config_manager.py` - Cache management (usually no changes)
- `logger_applications/AnkiLogger/config.json` - API URL config (no changes)
- `logger_applications/AnkiLogger/meta.json` - Cached bundles (DELETE for testing)

**Initialization:**
- `logger_applications/AnkiLogger/__init__.py` - Add-on entry point (no changes)
- `logger_applications/AnkiLogger/addon_manager.py` - Dependency injection (no changes)

### Documentation

- `CLAUDE.md` - Project overview and architecture
- `docs/ARCHITECTURE.md` - Detailed architecture diagrams
- `docs/statistics.md` - Statistic type behaviors and constraints
- `core/infrastructure/web_api/README.md` - API endpoint documentation

## Testing Checklist

Use this checklist to ensure your changes work:

**Build & Compile:**
- [ ] Rust code compiles: `cargo build -p lifestats-web-api`
- [ ] Rust code passes lint: `cargo clippy -p lifestats-application`
- [ ] Rust tests pass: `cargo test -p lifestats-application`
- [ ] Python code has no syntax errors (Anki will show errors on load)

**Provisioning:**
- [ ] Clear cached bundle from `meta.json`
- [ ] Start API server successfully
- [ ] Restart Anki and add-on loads without errors
- [ ] Open test deck and trigger provisioning
- [ ] Check API logs show `POST /integrations/anki` with your deck
- [ ] Verify response includes your new `*_id` field (5 fields total)
- [ ] Confirm AnkiLogger shows no error tooltips

**Logging:**
- [ ] Study some cards in the test deck
- [ ] Verify your logging code is called (add debug prints if unsure)
- [ ] Check API logs show `POST /statistics/{id}/log/{date}` for your metric
- [ ] Query the statistic via API: `curl http://localhost:3000/statistics/{id}`
- [ ] Verify logged value matches expected value

**Data Integrity:**
- [ ] Study multiple sessions and verify update policy works correctly
- [ ] Check that data appears in plots: `GET /statistics/{id}/plot/data`
- [ ] Verify multiple decks each get their own instance of the metric
- [ ] Confirm cached bundle in `meta.json` has correct structure

**Edge Cases:**
- [ ] Test with empty deck (no cards to study)
- [ ] Test with deck that's already finished
- [ ] Test provisioning same deck multiple times (should return same IDs)
- [ ] Test with API server offline (should show error tooltip but not crash)

## Troubleshooting

### Problem: "Provisioning still returns 4 metrics, not 5"

**Symptom:** API response has `time_spent_id`, `disruptions_id`, `backlog_id`, `finished_time_id` but not your new field.

**Cause:** Rust backend wasn't rebuilt or server wasn't restarted.

**Solution:**
```bash
cargo build -p lifestats-web-api
# Kill the running server (Ctrl+C)
cargo run -p lifestats-web-api
```

### Problem: "Python throws KeyError for new field"

**Symptom:** Anki shows error when loading add-on or when studying deck.

**Example error:**
```
KeyError: 'cards_studied_id'
```

**Cause:** Python is trying to use cached bundle that doesn't have the new field.

**Solution:**
1. Open `meta.json` in AnkiLogger add-on directory
2. Delete the `"deck_configs"` object entirely
3. Restart Anki
4. Provision will run again with new structure

### Problem: "Metric is created but no data is logged"

**Symptom:** Statistic exists in database but has no logged values.

**Cause:** Logging code is never called or fails silently.

**Debug steps:**
1. Add debug logging to your code:
   ```python
   print(f"DEBUG: About to log cards_studied: {cards_count}")
   ```
2. Check Anki's debug console: Tools → Add-ons → AnkiLogger → View Files → debug.log
3. Verify your logging code is in the right method (`ping()`, `stop_running()`, etc.)
4. Check API server logs for errors
5. Ensure `self._session_running` and `self._active_bundle` are truthy when expected

### Problem: "Update policy behaves unexpectedly"

**Symptom:** Values are added when you expected replacement, or vice versa.

**Cause:** Wrong update policy for your use case.

**Solution:**
- Review the policy table in section 5
- Common mistake: Using `sum` when you want `overwrite`
- Test with a fresh date to see policy in isolation
- Query the database to see actual stored values:
  ```sql
  SELECT * FROM logged_values WHERE statistic_id = <id> AND date = '2024-11-27';
  ```

### Problem: "Anki add-on won't load after changes"

**Symptom:** Anki shows error on startup, add-on doesn't appear in menu.

**Cause:** Python syntax error or import error.

**Solution:**
1. Check Anki's error dialog (should show traceback)
2. Open debug.log in add-on directory
3. Common issues:
   - Missing comma in dataclass
   - Incorrect indentation
   - Typo in imported names
4. Validate Python syntax:
   ```bash
   python3 -m py_compile session_tracker.py
   ```

### Problem: "API returns 500 error on provision"

**Symptom:** AnkiLogger shows error tooltip, API logs show internal server error.

**Cause:** Rust code has a runtime error (panic, unwrap on None, etc.).

**Solution:**
1. Check API server console for Rust backtrace
2. Common issues:
   - Typo in tag keys
   - Missing required metadata field
   - Wrong statistic type constructor
3. Run tests to catch errors earlier:
   ```bash
   cargo test -p lifestats-application
   ```

### Problem: "Different decks share the same statistic"

**Symptom:** Studying Deck A affects values for Deck B.

**Cause:** Tags are not unique per deck (missing or incorrect `deck_name` tag).

**Solution:**
- Verify tag structure in Rust code:
  ```rust
  .with_tag("source", "anki")
  .with_tag("deck_name", deck_name)  // This MUST be included
  .with_tag("metric", "cards_studied")
  ```
- The combination of all three tags ensures uniqueness

## Advanced Topics

### Conditional Logging

Sometimes you only want to log when certain conditions are met:

```python
def stop_running(self) -> None:
    if not self._session_running or not self._active_bundle:
        return

    # Only log cards_studied if at least 5 cards were reviewed
    cards_count = anki_utils.get_cards_studied_today(self._current_deck_id)
    if cards_count >= 5:
        self._api_client.log_statistic_value_async(
            statistic_id=self._active_bundle.cards_studied_id,
            date=self._get_today(),
            value=str(cards_count),
            update_policy="sum"
        )
```

### Rate Limiting / Throttling

To avoid excessive API calls, use throttling:

```python
from datetime import datetime, timedelta

class SessionTracker:
    def __init__(self, ...):
        # ... existing init ...
        self._last_cards_log = None
        self._cards_log_interval = timedelta(seconds=30)

    def ping(self):
        # ... existing logic ...

        # Only log cards_studied every 30 seconds
        now = datetime.now()
        if (self._last_cards_log is None or
            now - self._last_cards_log >= self._cards_log_interval):

            cards_count = anki_utils.get_cards_studied_today(self._current_deck_id)
            self._api_client.log_statistic_value_async(...)
            self._last_cards_log = now
```

### Per-Deck Metric Variations

By design, all decks get the same bundle structure. If you want deck-specific behavior:

**Option 1: Use tags to filter in queries**
```python
# Each deck automatically gets its own statistic instance
# Tags: source=anki, deck_name="Japanese", metric="cards_studied"
# vs.
# Tags: source=anki, deck_name="Spanish", metric="cards_studied"
```

**Option 2: Conditional logic based on deck properties**
```python
def _should_log_finished_time(self, deck_id: int) -> bool:
    """Only log finished_time for language decks, not math decks."""
    deck_name = anki_utils.get_deck_name(deck_id)
    return "Language" in deck_name or "Japanese" in deck_name
```

### Bundle Versioning

If you need to change the bundle structure in production without breaking existing data:

**Approach 1: Additive changes (recommended)**
- Add new metrics to the bundle (like this guide shows)
- Old cached bundles will re-provision and get new fields
- Data from old metrics is preserved

**Approach 2: Breaking changes (requires migration)**
1. Add `version` tag to all statistics:
   ```rust
   .with_tag("source", "anki")
   .with_tag("deck_name", deck_name)
   .with_tag("metric", "cards_studied")
   .with_tag("bundle_version", "2")  // NEW
   ```

2. Update `IntegrationService` to handle version lookup:
   ```rust
   // Look for version 2 first, fall back to version 1
   let tags_v2 = vec![
       ("source", "anki"),
       ("deck_name", deck_name),
       ("bundle_version", "2"),
   ];
   ```

3. Provide migration script to upgrade v1 → v2 data

### Error Handling Strategies

The add-on uses graceful degradation:

```python
try:
    # Attempt to log metric
    self._api_client.log_statistic_value_async(...)
except ApiConnectionError as e:
    # Network is down - log locally and retry later
    self._logger.warning(f"API unreachable: {e}")
    # Could implement local queue here
except ApiValidationError as e:
    # Bad data sent - this is a bug, log it
    self._logger.error(f"Invalid data: {e}")
except Exception as e:
    # Unknown error - log and continue (don't crash Anki)
    self._logger.exception(f"Unexpected error: {e}")
```

## Example: Complete Implementation

Let's walk through adding a "Cards Studied Today" metric end-to-end.

### Goal

Track how many cards are studied each day per deck. This differs from "Disruptions" (which counts sessions) and "Finished Time" (which is one-time). We want a daily total.

### Decisions

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| Metric Name | `cards_studied` | Clear, follows naming pattern |
| Statistic Type | `IntegerAmount` | It's a numeric count |
| When to Log | Session end | Avoids spamming API on every card |
| Update Policy | `sum` | Accumulate across multiple sessions |
| Data Source | `anki_utils.get_cards_studied_today()` | Query revlog table |

### Implementation

**Step 1: Rust - Update `anki_bundle_factory.rs`**

```rust
// File: core/application/src/use_cases/integration/anki_bundle_factory.rs

use anyhow::{anyhow, Result};
use lifestats_core::statistic::types::{
    AnyStatistic,
    IntegerAmountStatistic, IntegerAmountMetadata, AmountUpdatePolicy,
    // ... other imports ...
};

const ANKI_BUNDLE_METRICS: [&str; 5] = [
    "time_spent",
    "disruptions",
    "backlog",
    "finished_time",
    "cards_studied",  // ADDED
];

impl BundleFactory for AnkiBundleFactory {
    fn provision(&self, context: ProvisionContext) -> Result<Vec<ProvisionedStatistic>> {
        let deck_name = context.get("deck_name")
            .ok_or_else(|| anyhow!("deck_name is required"))?;

        let mut bundle = Vec::new();

        // ... existing time_spent, disruptions, backlog, finished_time ...

        // ADDED: Cards Studied metric
        let cards_studied_stat = IntegerAmountStatistic::new(
            IntegerAmountMetadata {
                name: format!("{} - Cards Studied", deck_name),
                description: Some(format!(
                    "Total number of cards reviewed in {}",
                    deck_name
                )),
                is_higher_better: Some(true),
                update_policy: Some(AmountUpdatePolicy::Sum),
            }
        )
        .with_tag("source", "anki")
        .with_tag("deck_name", deck_name)
        .with_tag("metric", "cards_studied");

        bundle.push(ProvisionedStatistic {
            key: "cards_studied".to_string(),
            statistic: AnyStatistic::IntegerAmount(cards_studied_stat),
        });

        Ok(bundle)
    }
}
```

**Step 2: Python - Update `api_client.py`**

```python
# File: logger_applications/AnkiLogger/api_client.py

from dataclasses import dataclass

@dataclass
class StatisticBundle:
    """Bundle of statistic IDs for an Anki deck."""
    time_spent_id: int
    disruptions_id: int
    backlog_id: int
    finished_time_id: int
    cards_studied_id: int  # ADDED
```

**Step 3: Python - Add data source in `anki_utils.py`**

```python
# File: logger_applications/AnkiLogger/anki_utils.py

import aqt
from datetime import datetime

def get_cards_studied_today(deck_id: int) -> int:
    """
    Get count of unique cards studied today for a specific deck.

    Queries Anki's revlog (review log) for cards reviewed since
    today's rollover cutoff.

    Args:
        deck_id: The Anki deck ID (did)

    Returns:
        Number of unique cards reviewed today
    """
    mw = aqt.mw
    if not mw or not mw.col:
        return 0

    # Anki's "today" starts at the cutoff time (default 4 AM)
    today_cutoff = mw.col.sched.day_cutoff
    yesterday_cutoff = today_cutoff - 86400  # 24 hours in seconds

    # Query revlog for distinct cards reviewed since yesterday's cutoff
    # Note: Anki's revlog.id is a timestamp in milliseconds
    result = mw.col.db.scalar(
        """
        SELECT COUNT(DISTINCT r.cid)
        FROM revlog r
        JOIN cards c ON c.id = r.cid
        WHERE c.did = ?
        AND r.id > ?
        """,
        deck_id,
        yesterday_cutoff * 1000  # Convert seconds to milliseconds
    )

    return result or 0
```

**Step 4: Python - Add logging in `session_tracker.py`**

```python
# File: logger_applications/AnkiLogger/session_tracker.py

def stop_running(self) -> None:
    """
    Stop the current session and flush all pending data.
    """
    if not self._session_running or not self._active_bundle:
        return

    self._logger.info("Stopping session")

    # Cancel timers
    if self._timeout_timer:
        self._timeout_timer.cancel()
        self._timeout_timer = None

    # Flush final time frame
    self._flush_time_frame()

    # Log final backlog
    backlog_count = anki_utils.get_total_backlog_count(self._current_deck_id)
    self._api_client.log_statistic_value_async(
        statistic_id=self._active_bundle.backlog_id,
        date=self._get_today(),
        value=str(backlog_count),
        update_policy="overwrite"
    )

    # ADDED: Log cards studied
    cards_count = anki_utils.get_cards_studied_today(self._current_deck_id)
    self._api_client.log_statistic_value_async(
        statistic_id=self._active_bundle.cards_studied_id,
        date=self._get_today(),
        value=str(cards_count),
        update_policy="sum"
    )

    # Clear state
    self._session_running = False
    self._start_time = None
    self._last_ping = None
    self._last_flush = None
```

**Step 5: Test**

```bash
# Terminal 1: Rebuild and run API
cargo build -p lifestats-web-api
cargo run -p lifestats-web-api

# Terminal 2: Clear cache
# Edit logger_applications/AnkiLogger/meta.json
# Delete the "deck_configs" object

# Restart Anki
# Study some cards in a test deck
# Check API logs:
# Should see: POST /integrations/anki → response includes cards_studied_id
# Should see: POST /statistics/{cards_studied_id}/log/2024-11-27

# Verify data
curl http://localhost:3000/statistics | jq '.[] | select(.name | contains("Cards Studied"))'
```

**Expected result:**
```json
{
  "id": 123,
  "name": "Japanese - Cards Studied",
  "type": "IntegerAmount",
  "description": "Total number of cards reviewed in Japanese",
  "logged_values": {
    "2024-11-27": "15"
  }
}
```

### What We Built

This complete implementation:
1. ✅ Defines the metric in Rust (backend)
2. ✅ Updates Python data structure (frontend)
3. ✅ Adds data source (Anki query)
4. ✅ Implements logging logic (on session end)
5. ✅ Uses correct update policy (`sum` for accumulation)
6. ✅ Handles errors gracefully

Now every Anki deck will automatically get a "Cards Studied" metric tracking daily review count!

## Next Steps

After successfully adding your metric:

1. **Monitor in production**: Watch for unexpected behavior or performance issues
2. **Create visualizations**: Use the plot API to visualize your new metric
3. **Set goals**: Use the goal API to set daily targets for your metric
4. **Share knowledge**: Update this doc if you discover new patterns or gotchas
5. **Iterate**: Based on usage, refine when/how the metric is logged

## Contributing

Found an issue with this guide or want to add clarifications?

1. This documentation lives at: `logger_applications/AnkiLogger/INTEGRATION.md`
2. Update it directly and commit your changes
3. Follow the commit conventions in `docs/COMMIT_CONVENTIONS.md`

## Additional Resources

- **LifeStats API Documentation**: `core/infrastructure/web_api/README.md`
- **Architecture Deep Dive**: `docs/ARCHITECTURE.md`
- **Statistic Type Reference**: `docs/statistics.md`
- **CQRS Pattern in Application Layer**: See command/query handlers in `core/application/src/use_cases/`
- **Anki Add-on Development**: https://addon-docs.ankiweb.net/
- **Hexagonal Architecture**: https://alistair.cockburn.us/hexagonal-architecture/
