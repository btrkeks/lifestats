# Deploy the ebook cleanup release

The initial cutover is complete on live master `5693f2f`. The laptop had no unsent
v7 rows and adopted 影響力の武器 into item 1. Its permanent prefix in
`ebook_books.legacy_spans` is `[[0,85900]]`.

The cleanup release implementation is complete on `feature/ebook-cleanup`. It removes the
one-time handover code. Deployment remains the coordinator's separately approved
action. Implementation and verification use copied databases and no devices.

## Release contents

- Both legacy POST endpoints return 410 with `client_upgrade_required` in the
  integration error envelope. Generic integration provisioning cannot create ebook
  statistics. The temporary legacy-books PUT route is removed.
- The startup migration drops `ebook_legacy_bindings`. It preserves `ebook_books`,
  its permanent prefix, the progress item's `reader_data_id` and `book_key` tags,
  statistic values, and the read-only `ebook_sync_events` history.
- The reader opens `books` at v9 and removes `ebookSyncState` and `ebookSyncQueue`.
  Sync DB v2 removes the unused `claims` store, and startup deletes the separate
  `ttu-lifestats-legacy` receipts database. Books, bookmarks, day records, producer
  identity, and cached book identities survive.
- The personal arm64 APK bundles this reader. Its version code is 20102, above
  20002. It uses `com.btrkeks.jidoujisho` and the existing personal signing key.

The build, APK hash, commit SHAs, and verification logs are recorded in
`/home/daniel/.local/state/pstack-t3/jidoujisho-lifestats/artifacts/cleanup/report.md`.
The retained APK is
`/home/daniel/.local/state/pstack-t3/jidoujisho-lifestats/artifacts/cleanup/jidoujisho-arm64-v8a-release.apk`.
No APK is installed by this cleanup task.

## Back up and capture the baseline

Before the approved deployment, pause reading so the before and after values are
comparable. Export the laptop library with ッツ's own export, including bookmarks.
Close every ッツ tab before loading the new browser build.

```sh
D=~/lifestats-backups/$(date +%F)-ebook-cleanup
mkdir -p "$D"
LIVE=/home/daniel/.local/share/src/lifestats/test_data/test_snapshot.db
sqlite3 "file:$LIVE?mode=ro" ".backup '$D/live.db'"
sqlite3 "$D/live.db" 'PRAGMA integrity_check;'
cp ~/.local/bin/lifestats-web-api "$D/lifestats-web-api.prev"
docker image tag ebook-reader-ebook-reader "ebook-reader-ebook-reader:prev-$(date +%F)"
curl -fsS 'localhost:3000/statistics/110/integer_amount/total?start_date=2026-01-01&end_date=2026-12-31' > "$D/total-before.json"
curl -fsS localhost:3000/progress-items/1 > "$D/item1-before.json"
```

The integrity check must print `ok`. Both JSON files must contain a response.

## Rebuild and deploy the server and docker reader

Merge the verified cleanup branch through the normal integration process. Build
both the server binary and the docker reader from that revision. Build Cargo from
the worktree, with an explicit temporary `DATABASE_URL`.

```sh
cd /home/daniel/.local/share/src/lifestats-ebook-cleanup
DATABASE_URL=$(mktemp /tmp/ebook-cleanup-build-XXXXXX.db) cargo build --release -p lifestats-web-api
install -m755 target/release/lifestats-web-api ~/.local/bin/lifestats-web-api
systemctl --user restart lifestats-web-api
cd logger_applications/ebook-reader
docker compose up -d --build
```

The server runs the table-drop migration at startup. Reopen the laptop reader at
`http://localhost:5001` and hard-reload once. If the old worker serves cached assets,
unregister that worker under DevTools Application and reload. Keep IndexedDB.
The Lifestats URL remains configured on the origin. No handover action is needed.

## Check the deployed release

- `GET /integrations/ebook` returns the same `server_id` and protocol 1.
- Compare the 2026 statistic total and item 1 with the captured JSON using `diff`.
  With reading paused, both must be unchanged.
- A read-only SQLite check shows no `ebook_legacy_bindings` table. `ebook_books`
  still contains the adopted row and `[[0,85900]]`.
- DevTools lists `books` at v9 and `ttu-lifestats` at v2. The panel has no old-work
  count. A failed book identity still appears as a book needing attention.
- After resuming reading, a report wholly inside the prefix adds zero. New reading
  beyond the prefix is credited once across producers.

## Recovery

Keep the previous server binary, docker image, database backup, library export,
and signing-key backup. The v8 reader cannot open a v9 profile. Use the library
export in a fresh profile if that reader must be restored.

Reverting the table-drop migration recreates an empty binding table. It does not
restore claim history. The permanent prefix remains in `ebook_books`. A full DB
restore requires a separate coordinated recovery because it discards every write
since the backup, including writes from other integrations.
