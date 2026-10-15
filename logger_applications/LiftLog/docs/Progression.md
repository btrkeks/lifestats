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

The keyboard's Enter key moves through a set the way Strong does. Enter in the weight cell saves the
weight and moves to the same set's reps. Enter in the reps cell does what ✓ does, then moves to the next
set's weight with the keyboard still open. If ✓ would refuse, focus goes to the cell to fix instead. On a
set that is already checked, Enter saves the edited reps and moves on without unchecking it. On the
exercise's last set, Enter logs it and closes the keyboard; it does not move to the next exercise. A cell
reached with Enter has its text selected, so typing replaces it.

**+ Add set** adds a set for today only. The plan keeps its set count, so the next workout has the
planned number of sets again. The extra set still counts as history for its position: if you add a
fourth set to a three-set plan and later change the plan to four sets, set 4 starts from it.

## Replacing an exercise

**Replace exercise** in an exercise's ⋯ menu opens the exercise search. The exercise you pick takes the
old one's place in the workout in progress, as if the workout had opened with it:

- **Every set is cleared, including sets you already checked.** The exercise keeps its number of sets,
  including sets you added, and each set starts from the new exercise's history as described above.
  Previous, the reps hint and `Aim` follow the new exercise.
- **The rest time and the superset link stay.** The old exercise's notes and link go, because they
  described the old exercise. A weighted exercise takes the resistance (external, bodyweight or none) of
  the last time you did the new one, and keeps the old one's if you never have.
- **A cardio exercise stays cardio** and keeps its set targets; incline and resistance come from the new
  exercise's latest time.
- **When checked sets disappear, the rest timer restarts from your newest remaining set**, or stops if
  there is none.

The plan does not change. Finishing the workout offers the swap as a plan change like any other edit.
Replace is not offered when you edit a past workout in History.

## Beating last time

In the workout in progress, each set row says how to beat its Previous. Nothing changes your weights or
plan, and nothing here shows when you edit a past workout in History.

- **An unchecked set shows `Aim 11+`** under Previous: the fewest reps at the row's weight that beat that
  Previous. Change the weight and the number follows as you type. Previous `60kg × 10` at 62.5kg reads
  `Aim 9+`, and at 80kg `Aim 1+`.
- **A checked set that beat its Previous shows `▲ Better`.** A set that matched or fell short shows
  nothing. A set checked with 0 reps never shows it.
- **✓ does not log the target.** Tapping ✓ without typing still logs the Previous reps, the hint in the
  reps cell. To log the target, type it.

How a set is compared with its Previous:

- **At the same weight, more reps win.** The target is one more rep. Weights in kg and lb count as the
  same when they convert exactly, and two sets at 0kg count as the same weight. For a bodyweight
  exercise this is the added weight, whatever you weighed in either workout.
- **At a different weight, the estimated one-rep max wins**, using the same formula as everywhere else in
  the app: `weight × (1 + reps / 30)`.
- **When the added weight of a bodyweight set changes, its load is the bodyweight of its own workout
  plus what you added**, which is negative for assistance. If either workout has no bodyweight, the set
  shows nothing.
- **A movement with no resistance compares reps only.**
- **The set shows nothing** when it has no Previous, when the resistance changed since, when the weight
  moved between 0 and a load, or when the target is more than 30 reps.
- **After a Previous of 0 reps there is no target.** A checked set at the same weight still shows ▲ for
  any reps. At a different weight it shows nothing, because a set of 0 reps has no estimated one-rep max.

## No gain in 3 workouts

An exercise card shows `No gain in 3 workouts` under its title when its best set has not improved three
workouts in a row. It reads the latest four finished workouts that have a checked set of that exercise,
not counting the one in progress. Each workout's best set is its highest estimated one-rep max, or its most
reps for a movement with no resistance. Doing the exercise twice in one workout counts once.

- **The flag shows** when each of the three newer workouts is no better than the one before it. A workout
  that improves on the one before breaks the run, even if it is still below an older best.
- **It does not show** with fewer than four such workouts, or when any of them cannot be compared because
  it mixed resistances, is a bodyweight exercise without a bodyweight, or moved no load. It also does not
  show when those workouts used a different resistance than today.
- **It goes away during the workout** as soon as a checked set beats the latest of those workouts.

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

For the code behind this: `readWeightedHistory` and `readRecentProgress` in `app/src/db/sessions.ts`,
the targets, ▲ and the stall rule in `app/src/models/set-progress.ts`,
`RecordedWeightedExercise.empty` in `app/src/models/session-models/recorded-weighted-exercise.ts`,
session start in `app/src/services/session-service.ts`, `Session.withReplacedExercise` in
`app/src/models/session-models/session.ts` with its picker in `app/src/hooks/useReplaceExercise.ts`,
and the set rows in `app/src/components/presentation/workout/weighted/set-rows.ts`.
