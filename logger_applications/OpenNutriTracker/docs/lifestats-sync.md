# Lifestats sync

The app can read data from a [Lifestats](../../../core/infrastructure/web_api/README.md)
web API, the habit and wellness tracker this repository is built around. Today
it reads one thing: daily weight from a Beurer scale, which fills the weight
log and, through it, the protein goal.

## Setting it up

Settings → Lifestats takes the server's base URL, for example
`http://100.64.0.1:3000` for a laptop on a Tailscale tailnet, or
`http://10.0.2.2:3000` for a server on the host of an Android emulator. An
address without a scheme gets `http://`. An empty address turns sync off.

The address, the last successful sync and the last failure are stored per
profile in `ConfigDBO` (`lifestatsBaseUrl`, `lifestatsLastSyncAt`,
`lifestatsLastErrorAt`, `lifestatsLastError`, `lifestatsLastErrorStatus`).
Saving an address clears the old status and syncs once.

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

## Reusing the client

`LifestatsClient` (`lib/core/data/data_source/lifestats/lifestats_client.dart`)
is not specific to weight. It reads the base URL from the config on every
request and offers `getJson`, `putJson` and `postJson` for any route, plus
`getDailyValues(statisticId, start, end)`. Every failure is a
`LifestatsException` with a `LifestatsErrorKind`: `notConfigured`,
`invalidBaseUrl`, `unreachable`, `timeout`, `http` (with the status) or
`badResponse`. A new sync, such as pushing daily caffeine or nicotine totals,
needs a use case that calls one route and records its own status.

The client sends plain `http://` deliberately. The AI clients'
`plaintext_destination_guard` refuses Tailscale's `100.64.0.0/10` range, but
Lifestats is the user's own server and a tailnet already encrypts the
traffic.
