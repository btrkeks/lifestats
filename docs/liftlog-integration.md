# LiftLog Integration

Lifestats logs gym workouts from our LiftLog fork
(`logger_applications/LiftLog`). The app pushes the finished workouts of every
day it changed to `PUT /integrations/liftlog/days`, and the server replaces
`[LiftLog] Workout Time` on exactly those days. The route reference, with the
request, the response, and the errors, is in
`core/infrastructure/web_api/README.md` under "Syncing LiftLog Workout Days".

## Statistics

Lifestats provisions the `liftlog` bundle on the first sync. It finds the
statistics by their full tag set (`source=liftlog`, `metric=<key>`,
`bundle_version=1`), so existing statistics are reused and the app never
configures an id.

| Key | Type | Name | Content |
|-----|------|------|---------|
| `workout_time` | `TimeSpans` | `[LiftLog] Workout Time` | Merged time frames of the day's finished workouts |
| `total_weight` | `IntegerAmount` | `[LiftLog] Total Weight Lifted (kg)` | Frozen. Still provisioned, but no route writes it since the push sync replaced the backup ingest. |

## Semantics

- **Replace the named days.** Each request lists days, and each day lists its
  finished workouts. The server rewrites the value for those days and leaves
  every other day alone. Resending a request changes nothing.
- **Clearing a day.** A day sent with `workouts: []` loses its value, so it
  reads like any other rest day (`has_value: false`). The app sends this after
  you delete or move the last workout of a day.
- **Day attribution** follows the date the app sends. Timestamps keep the
  device's local wall-clock time. A workout that crosses midnight is clamped
  into its date.
- **Short workouts** get a time frame of at least one minute so they stay
  visible.
- Overlapping or touching workouts on one day merge into one frame.

## Delivery and ordering

The app keeps a queue of changed dates in its own database and retries until
the server answers `200`. Each request carries the full current state of its
days, so a retry or a duplicate is harmless.

The server applies requests in arrival order and does not check versions. If
an old request is delayed long enough to arrive after a newer one for the same
day, the old state wins until the app sends that day again. We accept this
risk. The server writes in milliseconds, there is one phone, and the failure
needs a request stuck in flight while you edit the same day. The app's queue
makes sure every change is acknowledged. It does not order the server's
writes.

## Network

The API has no TLS and no authentication. The phone reaches it through
Tailscale Serve at `https://laptop-1.tail87da75.ts.net`, which proxies to
`localhost:3000` on the tailnet only. Do not expose the API to the public
internet.

## Implementation map

- Domain types: `core/application/src/use_cases/integration/liftlog_days.rs`
- Use case: `IntegrationService::sync_liftlog_days` and `day_time_frames` in
  `core/application/src/use_cases/integration/service.rs`
- Bundle: `core/application/src/use_cases/integration/liftlog_bundle_factory.rs`
- Per-day delete: `StatisticRepository::delete_log_value`
  (`core/core/src/statistic/ports/statistic_repository.rs`, SQLite in
  `core/infrastructure/persistence/src/repository.rs`)
- Route and request validation: `sync_liftlog_days` and `parse_liftlog_days` in
  `core/infrastructure/web_api/src/routes/integrations.rs`
- E2E test: `core/infrastructure/web_api/tests/liftlog_days_e2e.rs`

An earlier version received LiftLog's gzipped SQLite remote backups at
`POST /integrations/liftlog/backup`. That route could never clear a day, and
the app no longer produces that format, so it was removed.
