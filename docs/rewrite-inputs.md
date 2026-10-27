# Inputs for the Lifestats rewrite

Requirements and workarounds found in integrations that the rewrite should resolve. Add an entry when an
integration works around a gap in the current API. Remove it when the rewrite covers it.

## Integrations can't look up a statistic by a stable key

`GET /statistics` returns no tags, so a client that only reads a statistic can't find it by its
source tags. Only the source-owned `PUT` integration routes resolve statistics by tags.

- **Workaround:** OpenNutriTracker reads the Beurer weight with a hardcoded statistic ID
  (`weightStatisticId = 136` in
  `logger_applications/OpenNutriTracker/lib/core/domain/usecase/sync_lifestats_weight_usecase.dart`, branch
  `feature/ont-protein-target`). It breaks if the statistic is recreated or the database is rebuilt.
- **Needed:** a stable, documented way to address a statistic from a client, such as a lookup by tags
  (`source=beurer`, metric `weight`) or a durable key. Reads and writes should use the same key.

## Several timed amounts per day

`PointInTime` holds one time per day, so a second coffee overwrites or loses the first. Caffeine and
nicotine from OpenNutriTracker need a per-day list of `(time, amount)` entries, which the source replaces
day by day.

- **Status:** implemented on branch `feature/timed-amounts-statistic` (not yet merged into `master`).
  `TimedAmounts` stores each day as a list of `{time, amount}` entries, where `time` is `HH:MM:SS` or
  `null` when unknown and `amount` is an integer in the smallest unit (mg). OpenNutriTracker writes it only
  through `PUT /integrations/opennutritracker/days/{day}`, which replaces both caffeine and nicotine lists
  for the day; the app resends about the last 14 days on every sync. Readers see the daily total. Design and
  contract: `docs/plans/timed-amounts-statistic/`.
- **Rewrite should keep:** the list value shape, whole-day replace by the source, `[]` as a confirmed zero
  distinct from a never-sent day, and the daily total as the scalar projection.
- **Still open:** a "nothing after 14:00" cutoff goal, the logical-day boundary (a 00:30 entry counted
  toward the previous day sorts first), adding single entries by hand, and a derived "caffeine remaining
  at bedtime" statistic.
