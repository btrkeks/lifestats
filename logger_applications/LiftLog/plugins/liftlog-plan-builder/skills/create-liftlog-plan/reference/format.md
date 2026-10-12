# The `.liftlogplan` format

A `.liftlogplan` file is a single JSON object. `ProgramBlueprint.json` in this directory is the authoritative JSON Schema; this page explains it.

## Rules that catch everyone

Read these first. They are the reason most generated plans fail to import.

1. **Assume every field is required.** Only three are optional - `ceiling` and `onCeiling` on a progression rule, and `restBetweenSets` on a cardio set. Everything else must be present, including `notes` and `link` when empty - use `""`.
2. **`"version": 3`** goes on the root object, and **`"version": 6`** on every session. These are not the plan's own version number; they are the format's, and they differ from each other.
3. **Weights and steps are strings, not numbers.** `"step": "2.5"`, never `"step": 2.5`. Rep counts (`min`, `max`) are the exception: those are plain integers.
4. **Rests and times are ISO-8601 durations.** `"PT3M"` is three minutes, `"PT90S"` is ninety seconds, `"PT1M30S"` also works. A bare `"90"` or `90` is invalid.
5. **A weighted exercise has no set count.** One entry in `plannedSets` _is_ one set, so three sets means three identical `{ "reps": { "min": 0, "max": 0 } }` entries. Cardio exercises use `sets`, an array, one entry per interval.
6. **`type` values are case-sensitive**, and the casing is not consistent across the format. Exercise types use PascalCase (`"WeightedExerciseBlueprint"`); everything else - cardio targets, progression scopes, resistance - is lowercase or camelCase (`"time"`, `"allSets"`, `"bodyweight"`, `"allSetsMetTarget"`). Copy them exactly as written below.
7. **Write no rep targets and no progression rules.** LiftLog plans are a set count only; the app ignores rep targets, progression rules and `failureRest`. The format still requires the fields, so write `"min": 0, "max": 0`, `"progression": []`, and a `failureRest` equal to `maxRest`.

## Root

```json
{
  "version": 3,
  "name": "Push Pull Legs",
  "lastEdited": "2026-07-12",
  "sessions": [ ... ]
}
```

| Field        | Type   | Notes                                    |
| ------------ | ------ | ---------------------------------------- |
| `version`    | number | Always `3`.                              |
| `name`       | string | The plan name, as it appears in the app. |
| `lastEdited` | string | Date as `YYYY-MM-DD`. Use today's date.  |
| `sessions`   | array  | One per training day.                    |

## Session

```json
{
  "version": 6,
  "name": "Push",
  "notes": "Chest, shoulders and triceps.",
  "exercises": [ ... ]
}
```

| Field       | Type   | Notes                                              |
| ----------- | ------ | -------------------------------------------------- |
| `version`   | number | Always `6`. Not `3` - a session versions on its own. |
| `name`      | string | e.g. `"Push"`, `"Upper A"`, `"Leg Day"`.           |
| `notes`     | string | `""` if there's nothing to say.                    |
| `exercises` | array  | Weighted and cardio exercises can be mixed freely. |

## Weighted exercise

```json
{
  "type": "WeightedExerciseBlueprint",
  "name": "Barbell Bench Press",
  "plannedSets": [
    { "reps": { "min": 0, "max": 0 } },
    { "reps": { "min": 0, "max": 0 } },
    { "reps": { "min": 0, "max": 0 } }
  ],
  "restBetweenSets": { "minRest": "PT3M", "maxRest": "PT5M", "failureRest": "PT5M" },
  "supersetWithNext": false,
  "notes": "",
  "link": "",
  "resistance": "external",
  "progression": []
}
```

| Field              | Type    | Notes                                                                                                                   |
| ------------------ | ------- | ------------------------------------------------------------------------------------------------------------------------- |
| `type`             | string  | Exactly `"WeightedExerciseBlueprint"`.                                                                                  |
| `name`             | string  | The exercise name.                                                                                                      |
| `plannedSets`      | array   | One entry per set. See below.                                                                                           |
| `restBetweenSets`  | object  | See below.                                                                                                              |
| `supersetWithNext` | boolean | See supersets below.                                                                                                    |
| `notes`            | string  | Cues or instructions. `""` if none.                                                                                     |
| `link`             | string  | A URL explaining the movement. **Leave as `""` unless the user gave you a specific link** - do not invent or guess URLs. |
| `resistance`       | string  | Where the load comes from. See below.                                                                                   |
| `progression`      | array   | Always `[]`. The app ignores progression rules.                                                                         |

### Planned sets

`plannedSets` is only a set count: one entry per set, each `{ "reps": { "min": 0, "max": 0 } }`. Three
sets is three identical entries. The app does not prescribe reps; while logging, each set shows the
weight and reps from the last time that set was done, and the lifter changes the weight by hand.

If the user wants a rep scheme ("5x5", "3 x 8-12"), put it in the exercise's `notes`, not in
`plannedSets`.

### Resistance

Where the load on the movement comes from. One of:

| Value          | Use for                              | Effect                                                                                   |
| -------------- | ------------------------------------ | ---------------------------------------------------------------------------------------- |
| `"external"`   | Barbells, dumbbells, machines.       | The default. The weight logged is the weight lifted.                                     |
| `"bodyweight"` | Pull ups, dips, push ups.            | The weight logged is what is _added_ to the lifter (belt weight, or negative for assistance). |
| `"none"`       | Crunches, planks, air squats.        | No weight at all. The app hides weight while logging and in stats; reps are the whole story. |

Use `"none"` whenever added weight is not part of the exercise - it is better than `"external"` with a
weight of zero, which pollutes the stats with a zero-weight line.

### Rest

All three are required ISO-8601 durations.

| Field         | Meaning                                                   |
| ------------- | --------------------------------------------------------- |
| `minRest`     | The shortest acceptable rest.                             |
| `maxRest`     | The longest.                                              |
| `failureRest` | Unused by the app. Required; write the same as `maxRest`. |

Pick rests from the effort of the lift: heavy compounds 3–5 minutes, accessories 60–90 seconds.

### Supersets

There is no superset group. `"supersetWithNext": true` means "do this exercise back to back with the **next one in the array**, with no rest between them". To superset A with B, set it on A and leave it `false` on B. For a three-exercise circuit, set it `true` on the first two.

### Progression

Always `"progression": []`. The app no longer moves weights or reps on its own, and ignores any rules a
plan carries. Plans exported from older versions may hold rules; when editing such a plan, keep them as
they are unless the user asks otherwise.

## Cardio exercise

```json
{
  "type": "CardioExerciseBlueprint",
  "name": "Steady Run",
  "notes": "",
  "link": "",
  "sets": [
    {
      "target": { "type": "distance", "value": { "value": "5", "unit": "kilometre" } },
      "trackDuration": true,
      "trackDistance": true,
      "trackResistance": false,
      "trackIncline": false,
      "trackWeight": false,
      "trackSteps": false
    }
  ]
}
```

`sets` is an **array** here - one object per interval or round. There is no resistance or progression on cardio.

Every set needs a `target` plus all six `track*` booleans, which control the fields the app shows while logging. A set may also carry the same `restBetweenSets` object a weighted exercise takes; leave it out for steady-state work, where there is nothing to rest between. Turn on the ones that make sense for the movement (a bike: duration + resistance; a treadmill run: duration, distance, incline) and leave the rest `false`.

A target is one of:

```json
{ "type": "time", "value": "PT20M" }
{ "type": "distance", "value": { "value": "5", "unit": "kilometre" } }
```

Note the lowercase `type`. `unit` must be one of `"metre"`, `"yard"`, `"mile"`, `"kilometre"`, and `value.value` is a decimal **string**.

## Validate before you hand it over

```bash
node scripts/validate-plan.mjs "My Plan.liftlogplan"
```

No install, no network. It lists every problem with the path to the field, e.g.:

```
  plan/sessions/0/exercises/0/restBetweenSets/minRest must match format "duration"
  plan/sessions/1/exercises/0/progression/0/step must be string
```
