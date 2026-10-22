# Ebook reading API

`GET /integrations/ebook` returns `{"server_id":"<uuid>","protocol":1}`.
The UUID belongs to the database and survives process restarts. A client that sees
another UUID must deliver its records again.

## Report reading

`PUT /integrations/ebook/producers/{producer}/books/{book_key}/days/{day}`
accepts JSON with exactly these fields.

```json
{"title":"Book","length":10000,"spans":[[0,500]],"last_position":500,"observed_at":"2026-10-22T12:00:00Z"}
```

`producer` is a canonical lowercase UUID per reader origin database. `book_key` is
`ttu-text-v1:` followed by the lowercase SHA-256 hex digest of ttu's counted text.
Title and local book IDs do not determine identity. `length` is a positive integer
at most 2^53. The first report fixes the length for that key.

`day` is a strict valid YYYY-MM-DD label, calculated by the client from local time
with a 04:00 rollover. Days more than two days after the server's local date fail.
`spans` contains at most 4096 sorted, disjoint, non-adjacent half-open integer pairs
with `0 <= start < end <= length`. Empty spans are valid. `last_position` is an
integer in `[0,length]`. `observed_at` is RFC 3339. Titles are NFC-normalized and
trimmed, nonblank, and at most 1000 Unicode characters.

A successful report returns 204 after one immediate transaction commits the row,
credits, statistic, progress item, and checkpoint. Reports merge by union.
Identical replays and stale subsets change nothing. A newer observation updates
the position, even when it adds no spans. Existing item names are preserved.

Across all producers, each character belongs to its earliest reported day.
Earlier reading delivered late can move credits from a later day. For each day,
the statistic equals its residual plus the sum of book credits. The residual
includes existing legacy values and manual corrections. Legacy spans are excluded
from all new credits. Checkpoint ordering follows observation time.

## Adopt legacy books

After draining v7 events, send
`PUT /integrations/ebook/legacy-books/{reader_data_id}` with JSON
`{"book_key":"ttu-text-v1:<64 lowercase hex>","title":"Book","length":10000}`.
The positive reader ID selects a legacy item tagged `source=ebook` and
`progress_type=book`. Its normalized title must match. Inside the transaction,
the server reads its furthest checkpoint F, binds that existing item to the key,
and seeds `[0,F)` as legacy spans. Seeding changes no statistic values.
Repeated claims with the same identity return 204. Conflicting bindings fail.
If no legacy item exists, the claim records the key for a new item.

Before adoption, a day report whose title matches an unbound legacy item fails
with `legacy_adoption_pending`. A different book can sync normally.
The legacy POST `/integrations/ebook/events` and POST `/integrations/ebook` stay
available for one release so the laptop can drain old events exactly once.

## Errors

```json
{"error":{"code":"invalid_field","field":"spans","message":"Invalid spans"}}
```

| Status | Codes |
| --- | --- |
| 400 | `malformed_json`, `invalid_body` |
| 409 | `bundle_conflict`, `book_length_mismatch`, `legacy_adoption_pending`, `legacy_binding_conflict` |
| 415 | `unsupported_media_type` |
| 422 | `invalid_field`, `projection_overflow` |
| 500 | `storage_error` |

Unknown or duplicate fields and noninteger numeric literals fail validation.
Storage errors and validation failures commit nothing. Keep rejected reports;
retry network errors, 429 and 5xx with backoff, and park other errors visibly.

The complete served integration document is generated from utoipa annotations and
committed at `core/infrastructure/web_api/openapi.json`. The router test rejects
stale snapshots. Both implementations consume `span-cases.json` in this directory.
