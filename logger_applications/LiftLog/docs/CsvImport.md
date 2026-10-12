# Import from other apps

You can bring workout history from another app into LiftLog by importing a CSV export. Imported
workouts are added to History. Nothing already in LiftLog is deleted or overwritten.

This is not the same as [plaintext export](./PlaintextExport.md). LiftLog’s own CSV and JSON
exports cannot be imported back in.

## How to import

1. In the other app, export your workout history as a CSV (see [Supported formats](#supported-formats)
   for what the file should look like).
2. In LiftLog, go to **Settings → Export, backup, and restore → Import from other apps**.
3. Choose the matching **Format**, tap **Import**, and pick the file.

After a successful import, the workouts show up in History. If every workout in the file is already
there, LiftLog tells you nothing new was added.

## What to expect

- Your existing workouts and programs stay put. Import adds sessions, and a Strong import also adds
  a plan (see [Strong CSV](#strong-csv)).
- Importing the same file again does not create duplicates.
- A later export only adds workouts for days or slots that are not already in History. Changing
  sets or comments in the other app and exporting again does not update or duplicate a workout
  you already imported. Delete that workout in History and import again if you want the new
  version.
- If you delete an imported workout and import the same file again, it comes back.
- Only weighted sets are imported. Cardio-only or timed-only rows are skipped.
- Comments and notes from the file become exercise notes. 

## Supported formats

Choose the format in the app that matches the file you exported.

### Strong CSV

In Strong, export your workout data as CSV, then choose **Strong CSV** in LiftLog. It is the default
format.

Strong writes one row per set. LiftLog reads the `;`-delimited export with every field quoted, and
the `,`-delimited one that other Strong versions write. The weight header sets the unit:
`Weight (kg)` means kilograms and `Weight (lbs)` means pounds. A bare `Weight` header uses your app
unit preference.

- Each `Workout #` becomes one session. The workout name becomes the session name.
- The session starts at the workout's `Date` in your device's time zone.
- Numbered sets, drop sets (`D`), and failure sets (`F`) are imported. Warm-up sets (`W`) are not.
- Rows with empty reps are skipped, so timed or distance-only work is left out. A set with reps and
  no weight is imported at 0.
- No exercise gets a rep target. LiftLog matches history by exercise name and set number, so the
  in-workout "previous" numbers and prefilled weights pick up straight from your Strong history.
- The first `Rest Timer` row of an exercise sets its rest time.

The import also creates a plan named **Strong** from your templates. Each workout name that occurs
at least twice becomes one workout in the plan, ordered by name (DAY 1, DAY 2, …, DAY 10). Each one
copies the most recent workout of that name: its exercises in order, its number of working sets, and
its rest times. A workout name that occurs once, such as a one-off `New template`, is left out.

If your active plan is empty or is a built-in plan you never changed, the Strong plan becomes
active. Otherwise your active plan stays active, and you can switch to Strong under **Plans**.
Importing again replaces the Strong plan with one built from the new file, which undoes any edits you
made to it. If every workout in the file is already in History, nothing is imported, the plan
included.

| Imported | Not imported |
|----------|--------------|
| Workout name → session name | Warm-up sets (`W`) |
| Date → session date and start time | RPE |
| Exercise name, reps, weight | Distance and seconds of timed sets |
| Drop and failure sets | Duration |
| Set notes → exercise notes | Workout notes |
| Rest timer → plan rest time | |

### FitNotes-style CSV

Export a CSV from FitNotes for Android (not the `.fitnotes` database file).

The file should have a header row with at least **Date**, **Exercise**, **Weight**, **Weight Unit**,
and **Reps**. A typical FitNotes CSV export also includes Category, Distance, Distance Unit, Time, and
Comment.

- All sets on the same calendar day become one workout. 
- Weight units come from the file when present; otherwise LiftLog uses your app unit preference.
- **Imported:** date, exercise name, weight, weight unit, reps, comment.
- **Not imported:** category, distance, and time (cardio). Those columns can still be in the file.

### StrongLifts-style CSV

Export workout history as CSV from StrongLifts. Each row is one exercise; individual sets are
separate columns.

The file should have a date column `Date (yyyy/mm/dd)`, an `Exercise` column, and
pairs of `Set N (Reps)` / `Set N (KG)` or `Set N (LB)` columns (N is 1, 2, 3, …). Weight unit
is taken from those headers.

- Each date plus workout letter (A or B) is a separate session, so A and B on the same day stay
  separate.
- The workout name in the file becomes the session name in LiftLog.
- Sets with empty or **0 reps** are skipped (StrongLifts uses 0 for unused slots and failed sets).

| Imported | Not imported |
|----------|----------------|
| Date | Program name (not linked to a LiftLog plan) |
| Workout name → session name | Sets×Reps / Sets×Time summaries |
| Exercise name | Top set, e1RM, total reps/volume |
| Set reps + weight (per Set N columns) | Duration, start/end time |
| Notes → exercise notes | Timed-only work (`Sets×Time` without weight sets) |
| Body weight → session bodyweight | |

## How it works (for contributors)

The in-app **Read documentation** button opens this page; the sections above are
the user-facing guide.

Rows are parsed, grouped into sessions (see each format), then turned into `BackupData` and merged
with the same path as Restore (`importBackupData`). Session IDs are grouping-key derived: UUID v5
over the FitNotes calendar date, StrongLifts date + Workout id (A/B), or Strong
`strong|<Workout #>|<Date>` - not Workout Name, and not sets or notes. Re-import skips IDs already in History (first import wins). Changing sets or comments
in the source app and exporting again does not create a second session for that day; delete the
imported session and import again to refresh it. Stats invalidation (`setStatsIsDirty`) runs on the
external-import path only, not inside full backup restore.

Internal format ids (UI dropdown / `importFromExternal`): `Strong` | `FitNotes` | `StrongLifts`. These
are unrelated to plaintext export’s own `CSV` value.

Strong imports set `restSeconds` on each `NormalizedExercise` and `startedAt` on each
`NormalizedImportSession`. FitNotes and StrongLifts leave them unset, which gives default rest and noon
UTC timestamps. Every format writes set counts only: no rep targets and no progression rules. History
and plan build each exercise blueprint through the same function. The plan id is UUID v5 over the plan
name (`programIdFromCsvImport`).

```text
UI dropdown  ←── EXTERNAL_IMPORT_FORMATS (id, labelKey, import)
importFromExternal({ format })
  → pickFile
  → getExternalImporter(format)(bytes, { defaultWeightUnit })
       → format-specific parse + group → NormalizedImportSession[]
       → sessionsFromNormalized() → BackupData { workouts, programs }
       → (Strong) programFromNormalized() → programs: { [planId]: "Strong" plan }
  → filter session ids already in storedSessions
  → importBackupData(...)
  → setActivePlan(planId) if the active plan is empty or an untouched built-in
  → setStatsIsDirty(true)   // external path only
```

- Package: `app/src/services/csv-import/`
  - `external-import-formats.ts` — registry (`Strong` | `FitNotes` | `StrongLifts`)
  - `strong-csv.ts` / `fitnotes-csv.ts` / `stronglifts-csv.ts` — parse + normalize
  - `csv-to-sessions.ts` — `sessionsFromNormalized`, `sessionIdFromCsvContent`, `programFromNormalized`
  - `importers.ts` — `getImportForStrong` / `getImportForFitNotes` / `getImportForStrongLifts`
  - `csv-parse-utils.ts` — shared cell/number/Papa preamble helpers
- Settings effect: `import-external-effects.ts` uses the registry; filters existing session IDs;
  dispatches `importBackupData` + `setStatsIsDirty` (or already-imported snackbar). When the import
  carries a plan, it also dispatches `setActivePlan` if the active plan has no sessions or equals its
  `BuiltInPrograms` entry. `importBackupData`'s effect saves the plans synchronously inside that
  dispatch, so the plan exists before it becomes active.
