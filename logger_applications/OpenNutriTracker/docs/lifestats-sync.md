# Lifestats sync

The app syncs with a [Lifestats](../../../core/infrastructure/web_api/README.md)
web API, the habit and wellness tracker this repository is built around. It
reads daily weight from a Beurer scale, which fills the weight log and, through
it, the protein goal. It sends each day's caffeine and nicotine intakes, with
their times.

## Setting it up

Settings → Lifestats takes the server's base URL, for example
`http://100.64.0.1:3000` for a laptop on a Tailscale tailnet, or
`http://10.0.2.2:3000` for a server on the host of an Android emulator. An
address without a scheme gets `http://`. An empty address turns sync off.

The address, the last successful sync and the last failure are stored per
profile in `ConfigDBO` (`lifestatsBaseUrl`, `lifestatsLastSyncAt`,
`lifestatsLastErrorAt`, `lifestatsLastError`, `lifestatsLastErrorStatus`).
The caffeine and nicotine sync keeps its own status in the matching
`lifestatsIntake…` fields, so a working weight sync cannot hide a failing
push. Saving an address clears both statuses and runs both syncs once.

## Weight sync

`SyncLifestatsWeightUsecase` runs when the home screen opens and on every app
resume, at most once per 5 minutes, and from **Sync now** in Settings. It:

1. Reads statistic 136, `[Beurer] Weight (kg)`, for the last 30 days through
   `GET /statistics/136/achievements?start=…&end=…`. The value is an
   `IntegerAmount` in grams.
2. Writes each day that has a reading to the weight log as grams ÷ 1000 kg,
   replacing the entry for that date and keeping its note. A reading dated
   today also updates the profile weight, as a manual entry for today does.
3. Leaves days without a reading alone, so manual entries there survive, and
   skips days whose entry already holds the reading. A repeated sync writes
   nothing.

A background sync never shows an error. It records the failure, and
Settings → Lifestats shows it under the last sync time. **Sync now** reports
the result in a snackbar.

The weight log feeds the protein goal (protein g/kg × the 7-day average
weight), so a new weigh-in can change the target for days that are not
tracked yet. Days already tracked keep the goals saved when their first
item was logged.

## Caffeine and nicotine sync

`SyncLifestatsIntakeUsecase` runs on the same triggers as the weight sync (home
screen open, resume at most once per 5 minutes, **Sync now**) and fails as
quietly. It calls `PUT /integrations/opennutritracker/days/{day}`, whose
contract is in
[`docs/plans/timed-amounts-statistic/contract.md`](../../../docs/plans/timed-amounts-statistic/contract.md):

1. For each of the last 14 logical days, today included, oldest first, it sends
   one request with the day's complete `caffeine` and `nicotine` lists. The
   first PUT creates the two `TimedAmounts` statistics in Lifestats.
2. An intake contributes `amount × nutrient per 100 ÷ 100`, rounded to whole
   mg. Foods without the nutrient, or under half a mg of it, contribute
   nothing. A day without either sends `[]`, which Lifestats stores as a
   confirmed zero; a day never sent counts as a missed goal there.
3. `time` is the intake's local wall-clock `HH:MM:SS`. It is `null` only for
   intakes that know their day but not their time (a bare-midnight
   `dateTime`, from diary entries logged before intake times were recorded,
   or a date-only import).
4. Days are logical days: with a 04:00 day start, a 02:30 espresso goes out
   on the previous day with `"time": "02:30:00"`.

Because every sync resends the whole window, edits and deletes reach Lifestats
without the app tracking which days changed. Days older than 14 days keep the
last values sent. The first failed request stops the sync and is recorded.

## Reusing the client

`LifestatsClient` (`lib/core/data/data_source/lifestats/lifestats_client.dart`)
is not specific to weight. It reads the base URL from the config on every
request and offers `getJson`, `putJson` and `postJson` for any route, plus
`getDailyValues(statisticId, start, end)`. Every failure is a
`LifestatsException` with a `LifestatsErrorKind`: `notConfigured`,
`invalidBaseUrl`, `unreachable`, `timeout`, `http` (with the status) or
`badResponse`. A new sync needs a use case that calls one route and records
its own status, as the caffeine and nicotine sync does.

The client sends plain `http://` deliberately. The AI clients'
`plaintext_destination_guard` refuses Tailscale's `100.64.0.0/10` range, but
Lifestats is the user's own server and a tailnet already encrypts the
traffic.
