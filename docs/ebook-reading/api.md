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
with a 04:00 rollover. Days more than two days after the server's local date fail,
and so do days before 2026-10-01, when no reader used this protocol. Both answer 422
with field `day`. A device with a wrong clock parks its record instead of giving
that day the characters for good.
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

## Retired clients

`POST /integrations/ebook` and `POST /integrations/ebook/events` return 410 with
`client_upgrade_required` in the integration error envelope. They accept no writes,
including provisioning through the generic integration route. Upgrade the reader
to report spans. The temporary `PUT /integrations/ebook/legacy-books/{reader_data_id}`
route has been removed.

Previously adopted books retain their `legacy_spans` prefix permanently. Reading
inside that prefix earns no new credit. The `reader_data_id` and `book_key` progress
item tags remain. `ebook_sync_events` is retained as history with no writer;
`ebook_legacy_bindings` is dropped by the cleanup migration.

## Errors

```json
{"error":{"code":"invalid_field","field":"spans","message":"Invalid spans"}}
```

| Status | Codes |
| --- | --- |
| 400 | `malformed_json`, `invalid_body` |
| 409 | `bundle_conflict`, `book_length_mismatch` |
| 410 | `client_upgrade_required` |
| 415 | `unsupported_media_type` |
| 422 | `invalid_field`, `projection_overflow` |
| 500 | `storage_error` |

Unknown or duplicate fields and noninteger numeric literals fail validation.
Storage errors and validation failures commit nothing. Keep rejected reports;
retry network errors, 429 and 5xx with backoff, and park other errors visibly.

The complete served integration document is generated from utoipa annotations and
committed at `core/infrastructure/web_api/openapi.json`. The router test rejects
stale snapshots. Both implementations consume `span-cases.json` in this directory.
