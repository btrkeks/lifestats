# Docs index

Every doc in this directory, with a one-line description of what it covers. Skim this before starting
work to find the docs relevant to your area, and update it whenever you add, remove, or repurpose a doc.

## Architecture and patterns

- [Storage.md](./Storage.md) - the two on-device storage layers: preferences (`PreferenceService`, one
  file per key) and user data (SQLite via Drizzle). Both are injected into Redux effects via `extra`.
  Covers which to use, how to add to each, the startup hydration order, the SQL / JS / data migration
  order, why transactions must be synchronous, the relational session tables, and why Redux holds only
  the sessions being edited.
- [Migrations.md](./Migrations.md) - the `createMigrations()` chain in `app/src/models/storage/versions/`
  that brings previously-persisted JSON up to the shape the app expects. Sessions on the device are
  relational and change through SQL migrations instead. Read alongside `Storage.md`.
- [WorkoutWorker.md](./WorkoutWorker.md) - the platform-specific, message-driven execution environment
  for an in-progress workout (persistent notifications, background timers, system UI). Redux stays the
  source of truth; the worker is disposable.

## Features

- [Progression.md](./Progression.md) - how weights carry from one workout to the next: plans hold a
  set count only, each set starts at the weight last completed at its position (matched by exercise
  name), reps are suggested from history, and nothing progresses on its own. Covers the rep target and
  ▲ each set shows against its Previous in the live workout, replacing an exercise mid-workout, the
  "no gain in 3 workouts" flag, which history counts, the rest timer, and how old rep targets and rules are kept but ignored. Ends with
  pointers to the code.
- [PlanFileFormat.md](./PlanFileFormat.md) - the `.liftlogplan` file format, how plans are imported and
  exported, and the Claude skill that authors plan files against the schema.
- [PlaintextExport.md](./PlaintextExport.md) — CSV/JSON export of workout data, including which fields
  are included. Explicitly _not_ a backup mechanism; LiftLog cannot read these files back.
- [CsvImport.md](./CsvImport.md) — user guide for Import from other apps (Strong CSV with its plan,
  FitNotes-style and StrongLifts-style CSV); contributor notes at the end. Separate from plaintext export.
- [LifestatsSync.md](./LifestatsSync.md) - pushing each day's workout time to lifestats: setup, when
  days are sent and retried, what a day's time is, and for contributors the queue table, which writes
  queue a day, and the request contract.
- [CalendarSync.md](./CalendarSync.md) - moving the day's Gym event in the phone's calendar (Android)
  to each finished workout: setup, which workouts move which event, what Needs attention means, and for
  contributors the two tables, enrollment, one sync run, native occurrence writes, and explicit history
  preview, selection, and pinned backfill delivery.

## Generated

- [schemas/](./schemas) - JSON schemas generated from the app's models: `ai-plan/`,
  `program-blueprint/`, `workout-worker/`. Regenerate with `npm run json-schema` from `app/`; never
  hand-edit.
- [img/](./img) - images referenced by the docs above.
