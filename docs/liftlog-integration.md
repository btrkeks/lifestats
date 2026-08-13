# LiftLog Integration

Automatically logs gym workouts from the [LiftLog](https://github.com/LiamMorrow/LiftLog)
Android app into Lifestats — **without forking the app**. LiftLog's built-in
*remote backup* feature uploads a gzipped SQLite snapshot of its database
whenever data changed; Lifestats receives it at
`POST /integrations/liftlog/backup`, parses the `session` table, and rewrites
the workout statistics for every day in the snapshot.

## Statistics

Auto-provisioned on first upload (category `LiftLog`, tagged
`source=liftlog`, `bundle_version=1`):

| Key | Type | Name | Content |
|-----|------|------|---------|
| `workout_time` | `TimeSpans` | `[LiftLog] Workout Time` | Merged time frames from first to last set completion per session |
| `total_weight` | `IntegerAmount` | `[LiftLog] Total Weight Lifted (kg)` | Σ effective weight × reps (bodyweight-aware, pounds converted), rounded to whole kg |

## Semantics

- **Full-snapshot overwrite.** Every upload contains LiftLog's entire history,
  so each day's values are recomputed and blind-overwritten (the Fitbit-sync
  pattern). Re-uploading an identical snapshot changes nothing and publishes no
  events; edits to past workouts propagate on the next upload.
- **Day attribution** follows LiftLog's own `date` field (the local calendar
  date the session was started). Timestamps use the device's local wall-clock
  time; sessions crossing midnight are clamped into their attributed date.
- **Single-set sessions** get a minimum one-minute time frame so they stay
  visible.
- **Deletions:** a deleted session disappears from the next snapshot, which
  corrects days that still contain other workouts. A day whose *only* workout
  was deleted keeps its last logged values (the ingest never clears days).
- Sessions without a single completed set are skipped; unparseable session
  payloads are skipped with a warning instead of failing the upload.
- Legacy protobuf exports (from LiftLog's old export format) are rejected
  with `400` — only the modern SQLite remote-backup format is supported.

## LiftLog app setup

In LiftLog: *Settings → Backup and restore → Automatic remote backup*:

- **Endpoint:** `https://<your-server>/integrations/liftlog/backup`
  - LiftLog enforces `https://`. The Lifestats API itself has no TLS or auth,
    so the intended setup is Tailscale + `tailscale cert` (or any reverse
    proxy terminating TLS on the tailnet hostname). Do **not** expose the API
    to the public internet — it is unauthenticated.
- **API key:** optional in LiftLog; the server currently ignores the
  `X-Api-Key` header (access control is network-level via Tailscale).
- Use the **Test** button to force an immediate upload.

Upload timing: LiftLog uploads when the main session tab gains focus and the
data hash changed — i.e. shortly after you finish a workout and return to the
home screen, whenever the server is reachable. If the upload fails (e.g. no
tailnet connectivity), it retries on the next app open.

## Implementation map

- Parser (gzip + SQLite + session JSON): `core/infrastructure/liftlog_backup/`
  (`lifestats-liftlog-backup`, implements the `LiftLogBackupParser` port)
- Port + ingest use case: `core/application/src/use_cases/integration/`
  (`liftlog_backup_parser.rs`, `liftlog_bundle_factory.rs`,
  `service.rs::ingest_liftlog_backup`)
- Route: `core/infrastructure/web_api/src/routes/integrations.rs`
  (`ingest_liftlog_backup`, 32 MB body limit)
- Wiring: `lifestats-bootstrap/src/lib.rs`
- E2E test: `core/infrastructure/web_api/tests/liftlog_backup_e2e.rs`

LiftLog's wire format (for future maintenance): the body is
`Content-Type: application/octet-stream`, gzip magic `1f 8b`, decompressing to
a SQLite image (`SQLite format 3\0`) with table
`session(id TEXT PRIMARY KEY, payload TEXT)` where `payload` is versioned
`SessionJSON` (v4 at time of writing; v1 payloads lack the `version` field).
Set completions are ISO offset datetimes; weights are
`{unit: kilograms|pounds|nil, value: "<decimal string>"}`.
