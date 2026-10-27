# OpenNutriTracker contract

This is the contract the OpenNutriTracker sync client calls. The README section "Replacing OpenNutriTracker daily intake" (after "Replacing Yomikata daily totals") reproduces it.

## Replace one complete day

```
PUT /integrations/opennutritracker/days/{day}
Content-Type: application/json
```

`{day}` is a strict valid `YYYY-MM-DD` date: the day the app counts the entries toward. The server does not use receipt time, infer a timezone, or move entries across dates.

```json
{
  "caffeine": [
    {"time": "08:12:00", "amount_mg": 120},
    {"time": "13:40:00", "amount_mg": 60},
    {"time": null, "amount_mg": 40}
  ],
  "nicotine": []
}
```

| Field | Rule |
|---|---|
| `caffeine`, `nicotine` | Both required, non-null arrays, 0..1000 entries each. The pair is the app's complete day for both metrics. `[]` means confirmed zero intake. |
| entry | Object with exactly `time` and `amount_mg`. Unknown or duplicate keys fail, at every depth. |
| `time` | Required key. Exactly `HH:MM:SS`, `00:00:00`..`23:59:59`, the source entry's local wall-clock time. No fractions, offsets or leap seconds. `null` means the app knows the day but not the time (date-label entries). Never send a fabricated `00:00:00`. |
| `amount_mg` | Raw JSON integer, 1..9,007,199,254,740,991. `120.0`, `1e2`, strings, booleans, `0` and negatives fail. The client converts its doubles to whole mg and omits entries that round to 0. |
| daily sum | Per metric, checked sum ≤ 9,007,199,254,740,991. |
| order, duplicates | Order does not matter; the server sorts by (time, amount), unknown times first. Identical entries stay separate intakes; nothing is merged or deduplicated. |

## Success

`204 No Content`, empty body, for first provisioning, changed days, no-op replays and empty snapshots alike. The client treats any 204 as success without parsing a body.

## Errors

Scoped integration envelope; `field` is absent when no field applies:

```json
{"error":{"code":"invalid_field","field":"caffeine[0].time","message":"Expected HH:MM:SS local time or null"}}
```

| HTTP | code | Cases | `field` |
|---|---|---|---|
| 400 | `malformed_json` | Invalid JSON syntax | none |
| 400 | `invalid_body` | Body unreadable / over limit | none |
| 415 | `unsupported_media_type` | Missing or non-JSON Content-Type | none |
| 422 | `invalid_field` | Invalid path date | `day` |
| 422 | `invalid_field` | Missing, null or non-array metric; more than 1000 entries | `caffeine` / `nicotine` |
| 422 | `invalid_field` | Non-object root, unknown or duplicate root keys | none |
| 422 | `invalid_field` | Non-object entry, unknown/duplicate/missing entry keys | `caffeine[i]` |
| 422 | `invalid_field` | Invalid time | `caffeine[i].time` |
| 422 | `invalid_field` | Invalid amount | `caffeine[i].amount_mg` |
| 422 | `projection_overflow` | Daily sum exceeds the cap | `caffeine` / `nicotine` |
| 409 | `bundle_conflict` | Two statistics tagged with the same source/metric, one of the wrong type, or an explicit unit tag other than mg. Nothing is written. | none |
| 500 | `storage_error` | DB failure, corrupt stored list, store not configured. Nothing is committed. | none |

The whole body and date are validated before anything is provisioned. When several fields are invalid, which one is reported first is not part of the contract. SQL details are logged server-side only.

## Semantics

- **Atomic.** One SQLite `IMMEDIATE` transaction resolves or creates both statistics and writes both lists, or nothing happens.
- **Idempotent.** Repeating a request changes nothing and publishes no event. Edits and deletes propagate because the list *is* the day.
- **Provisioning.** The first PUT creates both statistics, even for an empty day:

  | Identity tags | Initial name | Category | Goal |
  |---|---|---|---|
  | `source=opennutritracker`, `metric=caffeine` | `[OpenNutriTracker] Caffeine (mg)` | OpenNutriTracker | none |
  | `source=opennutritracker`, `metric=nicotine` | `[OpenNutriTracker] Nicotine (mg)` | OpenNutriTracker | none |

  New statistics also carry `unit=mg` and `bundle_version=1` (descriptive, not identity). Identity is the source/metric tags, not the name or the ID. If you delete a statistic, the next PUT creates a new one. There is no `POST /integrations/opennutritracker`.
- **Other days** are never touched. Moving an entry to another day changes both days; inside the sync window both are resent anyway.
- **Missing vs empty.** A day the app never sent has no value and does not achieve a goal. A day sent as `[]` is stored, totals 0 mg and achieves an at-most goal.
- **Expected client behaviour: a fixed sync window.** On every sync the app sends the complete lists for each day of a fixed recent window (about the last 14 days, including today), one request per day. Days without intake arrive as `[]`, and edits and deletes inside the window propagate without the app tracking which days changed. Days older than the window keep their last synced value.
- **Ordering.** Last write wins. The client sends each day's requests in order, without concurrent retries, and retries with the current snapshot. The server does not reject stale requests (LiftLog precedent).
- **Ownership.** This PUT is the only writer. `POST /statistics/{id}/log/{date}` and `POST /statistics` return 400 for TimedAmounts, and the type is absent from `/statistics/loggable`. Goals are set through the normal goal endpoints.
- **Events.** `ValueUpdated` is published after commit, once per statistic whose day changed.

## Reading it back

| Read | Shape for a TimedAmounts statistic |
|---|---|
| `GET /statistics/{id}/logged/{day}` | `{"statistic_id":42,"date":"2026-10-27","has_value":true,"value":[{"time":null,"amount":40},{"time":"08:12:00","amount":120},{"time":"13:40:00","amount":60}]}`. Stored entries use the generic key `amount`. Confirmed zero: `has_value:true,"value":[]`. Never sent: `has_value:false`, no `value`. |
| `GET /statistics/{id}/achievements` | `value` is the daily total (`220`), `0` for `[]`, `null` when never sent. |
| dashboard `today_value` | Daily total. |
| plots (bar/line/scatter) | Daily total per day, goal target as a scalar line. `time_frames` and `circular_progress` return 400, even for ranges with no data. |
| goals | Same `goal_data` as IntegerAmount: `{"target":"400","is_at_least":"false"}` caps the day at 400 mg; stored as `{"target_amount":400,"is_at_least":false}`. Target 0..9,007,199,254,740,991. Never-sent days score 0%; `[]` scores like a total of 0. No goal is provisioned automatically. |
