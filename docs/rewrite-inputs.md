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

- **Status:** a `TimedAmounts` statistic type is being designed on branch `feature/timed-amounts-statistic`.
  If the rewrite lands first, it should carry this value shape and the per-day replace semantics.
