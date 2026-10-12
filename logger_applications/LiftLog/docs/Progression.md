# Progression - how your numbers carry from one workout to the next

LiftLog does not move your numbers for you. A plan says how many sets to do. Each set starts at the
weight you used for that set last time, and you change it yourself when you are ready to go up.

## What a plan holds

A weighted exercise in a plan has a name, a number of sets, a rest time, a resistance, notes and a link.
It has no rep target and no progression rules. Its summary reads `3 sets`, or `3 sets @ 50kg–60kg` on a
workout card once weights are known.

## Starting a workout

A workout from your plan opens each weighted exercise with the plan's number of sets, all unchecked.

- **Set 1 starts at the weight of the last set 1 you completed, set 2 at the last set 2, and so on.**
  The sets do not have to come from the same workout. If your latest workout skipped set 2, set 2 comes
  from the workout before that.
- **A set you have never completed starts at the weight of the nearest earlier set that has one.** With
  history for sets 1 and 2 only (52kg and 55kg), a four-set plan opens at 52, 55, 55, 55.
- **With no history at all, every set starts at 0** in your preferred unit.
- **Reps are never filled in.** The reps you did last time show as a suggestion instead (see below).

## During a workout

Each set row shows **Previous**: the weight and reps from the last time that set was completed. Its reps
also appear as the hint in the reps cell. Tapping ✓ without typing logs the hinted reps at the row's
weight; with no hint, ✓ moves you to the reps cell instead.

**+ Add set** adds a set for today only. The plan keeps its set count, so the next workout has the
planned number of sets again. The extra set still counts as history for its position: if you add a
fourth set to a three-set plan and later change the plan to four sets, set 4 starts from it.

## Which history counts

- **Matching is by exercise name.** Case, surrounding spaces and a trailing `s` or `es` are ignored, and
  `flies`, `flyes` and `flys` match. Changing the set count or the resistance keeps the history; renaming
  the exercise starts over. A weighted and a cardio exercise never share history, even with the same name.
- **Only finished workouts count.** The workout in progress never feeds the next one.
- **Only checked sets count.** A weight typed into a set you never checked is ignored. A set checked
  with 0 reps counts.
- **Editing a past workout in History** shows the latest other finished workout as its previous
  numbers, even one done later.
- **The last time's notes** are the notes of the workout that holds your most recent completed set of
  that exercise, even when they are empty.

## Rest

The rest timer runs from the exercise's minimum to its maximum rest after every set. There is no longer
a longer rest after a failed set, because there is no target to fail.

## Plans and data from before

Plans, workouts and backups written by older versions still store rep targets, progression rules and a
failure rest. LiftLog reads and writes them unchanged, so nothing is lost and old backups still restore,
but nothing uses them: they do not show, do not move any weight and do not change the rest timer. This
includes the progression rules in the built-in plans and in imported `.liftlogplan` files.

## Cardio

Cardio exercises work as before. Each set starts with the incline and resistance of the latest time you
did that exercise, under exactly the same name and with the same kind of first-set target (time or
distance), including the workout in progress.

---

For the code behind this: `readWeightedHistory` in `app/src/db/sessions.ts`,
`RecordedWeightedExercise.empty` in `app/src/models/session-models/recorded-weighted-exercise.ts`,
session start in `app/src/services/session-service.ts`, and the set rows in
`app/src/components/presentation/workout/weighted/set-rows.ts`.
