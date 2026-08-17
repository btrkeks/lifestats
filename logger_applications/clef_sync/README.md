# Clef practice activity sync

This logger pulls Clef's authoritative, idle-capped daily practice totals over
SSH and writes whole minutes to an existing Lifestats `IntegerAmount`. Clef
remains the only practice clock; this importer only copies absolute daily
totals with `update_policy: overwrite`, so hourly retries cannot double-count.

The default seven-day rolling window repairs late session finalization, missed
runs while this machine was asleep, and activity around Clef's 03:00 logical
day rollover. Only days containing a Clef session are sent. Lifestats receives
the same floored whole-minute value shown in Clef, while the export retains
exact `activeMillis` for future consumers.

## Requirements

- A deployed Clef binary with `practice-activity export`.
- Non-interactive SSH access to the Clef host. The configured SSH account must
  be able to run `sudo -n -u clef` for the exporter command.
- `bash`, `curl`, `jq`, and OpenSSH on the Lifestats machine.
- The Lifestats API reachable locally, normally at `http://localhost:3000`.

## One-time setup

Create a dedicated integer statistic and note the returned `id`:

```bash
curl --fail --silent --show-error \
  -X POST http://localhost:3000/statistics \
  -H 'Content-Type: application/json' \
  --data '{
    "name": "[Clef] Piano practice (minutes)",
    "category": "Piano",
    "statistic_type": "IntegerAmount"
  }'
```

Install the importer, configuration, and user units from the Lifestats checkout:

```bash
install -Dm755 logger_applications/clef_sync/clef-lifestats-sync \
  "$HOME/.local/bin/clef-lifestats-sync"
install -Dm600 logger_applications/clef_sync/clef-sync.env.example \
  "$HOME/.config/lifestats/clef-sync.env"
install -Dm644 logger_applications/clef_sync/systemd/lifestats-clef-sync.service \
  "$HOME/.config/systemd/user/lifestats-clef-sync.service"
install -Dm644 logger_applications/clef_sync/systemd/lifestats-clef-sync.timer \
  "$HOME/.config/systemd/user/lifestats-clef-sync.timer"
```

Edit `~/.config/lifestats/clef-sync.env` with the Clef account email and the
actual Lifestats statistic id. The importer deliberately ignores user and
system SSH configuration so the systemd service is deterministic. Default SSH
identity files still work; otherwise set the key's absolute path in
`CLEF_SYNC_SSH_IDENTITY_FILE`.

Verify the exact remote edge before enabling automation:

```bash
ssh root@app.playclef.com \
  "sudo -n -u clef /opt/clef/bin/clef practice-activity \
   --db /var/lib/clef/memorization.sqlite export \
   --email you@example.com --utc-offset-minutes 120 --days 7"

systemctl --user daemon-reload
systemctl --user start lifestats-clef-sync.service
journalctl --user -u lifestats-clef-sync.service -n 20
systemctl --user enable --now lifestats-clef-sync.timer
```

The offset is minutes east of UTC, matching Clef's browser protocol. When
`CLEF_SYNC_UTC_OFFSET_MINUTES` is omitted, the importer derives the machine's
current offset on every run, including daylight-saving changes.

## Operations

```bash
systemctl --user start lifestats-clef-sync.service
systemctl --user list-timers lifestats-clef-sync.timer
journalctl --user -u lifestats-clef-sync.service
```

The importer validates that the configured Lifestats statistic exists and is
an `IntegerAmount` before contacting Clef. It also rejects unknown Clef export
schema versions or malformed rows before writing anything.
