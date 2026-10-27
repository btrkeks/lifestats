# Fork roadmap

Plans for Daniel's fork of OpenNutriTracker inside Lifestats. Upstream's own analysis is in
[competitive-feature-gaps.md](competitive-feature-gaps.md); this page lists only what the fork intends to do.

## Adaptive calorie target

Estimate the real daily energy expenditure from logged intake and the weight trend, and set the calorie
target from that estimate instead of the IOM 2005 formula.

**Why:** the formula multiplies a resting estimate by an activity level the user guesses, and logged
activities add their own rough estimates on top. Both are guesses, and training can be counted twice: once
in the chosen activity level and again as a logged workout. Activity estimates also have a large error
(Shcherbina et al. 2017: wrist trackers miss energy expenditure by 27–93%), and the body offsets about a
quarter of exercise energy (Careau et al. 2021, already used in `workout_compensation_calc.dart`). Intake
compared with the weight trend measures the actual result and replaces both guesses.

**Inputs the fork already has:** a daily weigh-in from the Beurer scale, synced through Lifestats into the
weight log (`feature/ont-protein-target`), and logged intake.

**Design notes:**

- Smooth the weight first. Daily weigh-ins swing by water; reuse or extend `weight_moving_average.dart`.
- Estimate over a 2–3 week window. Expenditure ≈ mean daily intake − (weight-trend change × energy per kg)
  ÷ days. Treat the energy per kg (about 7,700 kcal) as an approximation.
- Change the target slowly, at most once a week and by a bounded step. Upstream names "whipsawing" the
  target week to week as the main failure mode.
- Only trust days that were logged completely. Unlogged or partly logged days bias the estimate low; there
  must be a way to mark or detect incomplete days.
- Fall back to the formula until there is enough data, and show which source the target came from.
- Prototype it as a pure calculation in `lib/core/utils/calc/` against seeded demo data and Daniel's real
  history before wiring it into goals.

**Depends on:** consistent logging. The quick-log widget and AI-assisted logging come first.

**Until then:** choose the activity level for a normal week including training, and don't log workouts in
this app. LiftLog already records training.

## Other planned work

- **Protein target from body weight:** done on `feature/ont-protein-target` (1.8 g/kg × 7-day average
  weight, Beurer weight synced from Lifestats).
- **Caffeine and nicotine:** done on `feature/ont-caffeine-nicotine`. Caffeine and nicotine are
  nutrient fields (caffeine also from Open Food Facts), intake times are recorded at save, shown on
  each row and editable, and the last 14 days sync to Lifestats' `TimedAmounts` statistics
  ([lifestats-sync.md](lifestats-sync.md)). The "my coffee" and nicotine-gum foods are custom foods.
- **AI-assisted logging:** photo plus weighed grams plus a description, in one request. Builds on
  upstream's AI meal logging ([ai-architecture.md](ai-architecture.md)), which drops gram amounts on the
  photo path today.
- **Sync through a laptop relay:** the food log as events, with the phone as an offline replica, in the
  style of Yomikata's relay. Unlocks a web UI, agent logging, and daily totals in Lifestats.
- **Cleanup:** hide meal types (fill them from the time of day), and consider removing activity tracking,
  fasting, and onboarding.
