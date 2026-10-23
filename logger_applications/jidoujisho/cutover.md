# Cutover runbook: ebook reading sync

Every step here touches live data or live services. Run it only after you approve it. Each step has its own check and its own recovery. Do not read in any ッツ reader between step 1 and the end of step 3, so the before and after numbers stay comparable.

Code lives on Lifestats `feature/ebook-reading-sync` in `~/.local/share/src/lifestats-ebook-sync`. The fork is `logger_applications/ebook-reader`. The personal Android app is `logger_applications/jidoujisho`, vendored from upstream `cc7e4925` with all nine host commits through `eeb94512`. Use the final vendor report for the delivery commit and APK hashes.

## 1. Baseline and backups

```sh
D=~/lifestats-backups/$(date +%F)-ebook-sync; mkdir -p "$D"
LIVE=/home/daniel/.local/share/src/lifestats/test_data/test_snapshot.db
sqlite3 "file:$LIVE?mode=ro" ".backup '$D/live.db'" && sqlite3 "$D/live.db" 'PRAGMA integrity_check;'
cp ~/.local/bin/lifestats-web-api "$D/lifestats-web-api.prev"
docker image tag ebook-reader-ebook-reader "ebook-reader-ebook-reader:prev-$(date +%F)"
curl -s 'localhost:3000/statistics/110/integer_amount/total?start_date=2026-01-01&end_date=2026-12-31' > "$D/total-before.json"
curl -s localhost:3000/progress-items/1/summary > "$D/item1-before.json"
```

In the laptop reader at `http://localhost:5001`, use ッツ's own export to save the library (books and bookmarks) to `$D`. The new reader upgrades the browser database to v8, which the old reader cannot open, so this export is the only way back for the browser side.

**Check:** the integrity check prints `ok`, and both JSON files are non-empty.

## 2. Deploy the server

1. Merge `feature/ebook-reading-sync` into `master`. The branch is based on `ea68f0c`, and `master` has gained LiftLog merges since then. Neither touches the other's files.
2. From the **worktree**, run `cargo build --release -p lifestats-web-api`. Never build from the main checkout: its `.env` points at the live DB.
3. Install and restart:
   ```sh
   install -m755 target/release/lifestats-web-api ~/.local/bin/lifestats-web-api
   systemctl --user restart lifestats-web-api
   ```
   The new migrations run on startup.

**Check:**
- `curl -s localhost:3000/integrations/ebook` returns `{"server_id":"…","protocol":1}`.
- The total and item 1 summary are byte-identical to `total-before.json` and `item1-before.json`, compared with `diff`, not "or more".

**Recovery:** reinstall `$D/lifestats-web-api.prev` and restart. The new tables stay unused. Restore `$D/live.db` only as a last resort, with the service stopped, and only after accepting that it discards every write since step 1 from every integration (Yomikata, Clef and others).

## 3. Laptop browser reader (docker on :5001)

1. Rebuild `logger_applications/ebook-reader` from the merged `master` with `docker compose up -d --build` in that directory.
2. Close every ッツ tab, then open `http://localhost:5001` and hard-reload once (Ctrl+Shift+R). The browser build keeps upstream's service worker, which can serve the old assets on the first load.
3. Confirm the new client is the one running. In DevTools, `indexedDB.databases()` must list `books` at version 8 and a `ttu-lifestats` database. If not, unregister the worker in DevTools under Application → Service workers, which keeps IndexedDB, and reload.
4. Open the Lifestats panel and set `http://localhost:3000`. Old unsent rows drain there with their original event ids. If you ever point this profile at another server, they are resent, and the server's event-id check keeps that safe.
5. Read the panel's details. A book listed as blocked with `not_in_library` is a book the old client tracked but that is no longer in the library. Re-import that EPUB into the laptop reader and it gets claimed normally. This is a known limit of the claim code, which needs the local book to compute its key.

**Check:**
- The panel shows synced, with no blocked books.
- Item 1 is linked:
  ```sh
  curl -s localhost:3000/progress-items/1 | jq .tags.book_key   # "ttu-text-v1:…"
  sqlite3 "file:$LIVE?mode=ro" "SELECT book_key, legacy_spans FROM ebook_books;"
  ```
  Note the adopted prefix. The rehearsal on a backup showed `[[0,85900]]`, but trust the live value.
- The total equals `total-before.json`, plus any old rows that were never delivered.

**Recovery:** `docker compose down`, then start `ebook-reader-ebook-reader:prev-<date>`. The browser database is now v8, so the old reader shows an empty library until you import the step-1 export into a fresh profile.

## 4. Personal release build and phone

The phone was off limits during vendor implementation. Every command below is a
future cutover instruction for the operator. Hold the shared `artifacts/phone.lock`
for device operations and pass `-s GQHE9HZHLVRGQOAM` on every adb command.

Every phone step requires the phone unlocked and the relevant app in foreground
focus, except the three file/package operations `adb push`, `adb install`, and
`adb uninstall`. Do not drive the UI while another app or person is using it.

1. Back up `~/.local/share/keystores/jidoujisho/` securely, including `release.jks`
   and `key.properties`. Keep their permissions private. Losing the key means
   future updates require an uninstall, which loses local app data. This laptop
   directory alone is not a backup. Check the secure backup can restore both files.
2. Build from the Lifestats repo. No phone focus is needed for this laptop step.
   The default signing properties path is absolute. An alternative can be supplied
   with `JIDOUJISHO_KEY_PROPERTIES`.
   ```sh
   source ~/.local/state/pstack-t3/jidoujisho-lifestats/artifacts/harness/env.sh
   cd ~/.local/share/src/lifestats-ebook-sync/logger_applications/jidoujisho
   python3 tool/ttu_bundle/update_ttu_bundle.py --verify
   flutter pub get --enforce-lockfile
   GRADLE_OPTS='-Dorg.gradle.jvmargs=-Xmx4096m -Dorg.gradle.workers.max=1' \
     flutter build apk --release --split-per-abi --target-platform android-arm64 --no-pub
   ```
   The build output is
   `/home/daniel/.local/share/src/lifestats-ebook-sync/logger_applications/jidoujisho/build/app/outputs/flutter-apk/app-arm64-v8a-release.apk`
   (268 MB, version code 20002). Always build with `--split-per-abi`; see the README. The retained arm64 release is
   `/home/daniel/.local/state/pstack-t3/jidoujisho-lifestats/artifacts/vendor/jidoujisho-arm64-v8a-release.apk`.
   Check `aapt2 dump badging` shows `com.btrkeks.jidoujisho` and
   `apksigner verify --print-certs` matches release certificate SHA-256
   `6a6dfe0c69b30fc0c3028bc5089691ca726ab64de3c3dec268296718bb14e15e`.
   `python3 tool/android/verify_release.py --certificate-sha256 <fingerprint> <apk>`
   checks identity, signer, and bundled assets together. Do not install an x86_64 APK on the phone.
3. Uninstall the old **test install**, which holds only test data from isolated
   servers. This package operation needs no foreground focus.
   ```sh
   adb -s GQHE9HZHLVRGQOAM uninstall app.arianneorpilla.yuuna
   ```
   Do not use `pm clear`. The daily old app `app.lrorpilla.yuuna` stays installed
   until the user has verified the new app.
4. Install the retained arm64 release as the new app. This package operation
   needs no foreground focus.
   ```sh
   adb -s GQHE9HZHLVRGQOAM install -r ~/.local/state/pstack-t3/jidoujisho-lifestats/artifacts/vendor/jidoujisho-arm64-v8a-release.apk
   ```
   The new package is `com.btrkeks.jidoujisho`. Its launcher label remains
   `jidoujisho`, so distinguish it from the old app through Android App info.
   Its native activity class remains `app.arianneorpilla.yuuna.MainActivity`.
5. With the new app in foreground focus, complete first-run setup, grant the
   required storage permissions, and select the Reader tab. Enable Auto Bookmark
   in the reader settings. Set up AnkiDroid access, deck, note type, fields, and
   mining options from scratch. Do not assume old Anki configuration migrated.
6. Only the real book below is needed on the phone. Pushing it needs no foreground
   focus.
   ```sh
   adb -s GQHE9HZHLVRGQOAM push '/home/daniel/main/日本語/本/影響力の武器［新版］　人を動かす七つの原理.epub' /sdcard/Download/
   ```
   With the new app in foreground focus, open the ッツ library manager and import
   that EPUB from Downloads. Do not import the test fixture or the whole old
   library. Jump to the user's last reading position. A jump records no reading.
7. With Android Settings/Tailscale in foreground focus, enable the tailnet.
   Return the new app to foreground focus. Open its Lifestats panel, set
   `http://laptop-1:3000`, and run **Test** before reading. The emulator verification
   used USB reverse to an isolated server, so verify tailnet reachability here.
8. With the new reader in foreground focus, run the reading checks below. Then
   verify an Anki lookup and a mined card with AnkiDroid in foreground focus.
9. Only after the new app, reading sync, current position, and Anki setup are
   verified, the **user** uninstalls the old daily app `app.lrorpilla.yuuna`.
   If using `adb uninstall`, that package operation needs no foreground focus.
   If using Android Settings, keep Settings in foreground focus. This is a user
   cleanup step, not an implementation-agent action.

**Check:**
- With the new reader in foreground focus, read one page of 影響力の武器 inside
  the adopted prefix noted in step 3. The day's statistic must not rise.
- With the new reader in foreground focus, read a page beyond that prefix.
  The statistic must rise by that page's newly read characters once.
- The position survives closing and reopening the new reader in foreground focus.
- A lookup opens and a mined card reaches the newly configured Anki deck.

**Recovery:** keep the old daily app until these checks pass. If abandoning the
new app, uninstall only `com.btrkeks.jidoujisho`; this package operation needs no
foreground focus. The server retains reading already delivered. Uninstalling the
new app destroys its local books, bookmarks, and unsent records, so export needed
local data with the new app in foreground focus before that uninstall.

## 5. Later: the cleanup release

Do this once the laptop panel shows no old work and no blocked books, and item 1 is linked:

- replace `POST /integrations/ebook/events` and `POST /integrations/ebook` with 410 `client_upgrade_required`
- delete the legacy-books claim route and the fork's `legacyCutover` module
- upgrade `books` to v9, which drops the v7 stores
