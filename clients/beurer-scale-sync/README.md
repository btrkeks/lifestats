# Beurer scale sync

Reads a Beurer BF500 bathroom scale over Bluetooth LE and sends each day's first weigh-in to
Lifestats (`PUT /integrations/beurer/days/{day}`): weight, body fat, muscle and body water.
The phone app is not involved. The laptop registers its own user slot on the scale.

Every reading is also appended to `~/.local/share/lifestats-beurer/readings.jsonl`, so nothing
is lost while Lifestats is down. `~/.local/share/lifestats-beurer/synced-days.json` records what
Lifestats acknowledged; delete a day from it to resend that day.

## How a weigh-in reaches Lifestats

1. You weigh yourself. The scale recognises its only user (this laptop's slot) and stores the
   reading.
2. Every 20 minutes the service connects, sets the scale's clock, gives the consent code for its
   slot, receives the stored readings and disconnects, usually within 10 seconds.
3. It logs the readings and sends any day whose first weigh-in changed.

The BF500 stays connectable around the clock and its advertisements look the same before and
after a weigh-in, so the service collects on a timer. It never asks the scale to measure: that
lights the display, and doing it on every connection kept the display on permanently.

The laptop must be in Bluetooth range at some point after the weigh-in. Readings wait on the
scale until then. While the service is connected the scale skips its own weigh-in, so a weigh-in
that starts during those few seconds shows APP; step off and on again.

## One-time setup

```sh
uv run beurer-scale-sync register --address D8:0B:CB:5F:A5:2D   # step on the scale
uv run beurer-scale-sync pair                                    # press SET when the scale shows PAIR
sudo ./relabel-bond.sh D8:0B:CB:5F:A5:2D
cp ../../systemd/lifestats-beurer-sync.service ~/.config/systemd/user/
systemctl --user enable --now lifestats-beurer-sync.service
```

`register` copies the profile (birth date, sex, height, activity level) of the scale's first user
into the new slot; the scale needs it for body composition. The slot number and consent code are
saved to `~/.config/lifestats-beurer/state.json`.

Then delete every other user slot on the scale, for example in the Beurer app (Device settings,
User settings; the phone needs Bluetooth on). Otherwise the scale files weigh-ins under the other
slot, or alternates "U: 1" and "U: 2" when two profiles match, and the laptop never sees them.

## Why `relabel-bond.sh` exists

The BF500 pairs with the debug public key from the Bluetooth Core specification, and it requires
numeric comparison, so each pairing needs SET pressed on the scale. Linux deletes debug-key
pairings when the pairing ends and skips them when loading keys
(`net/bluetooth/smp.c`, `mgmt.c`), so without the relabel every connection would ask for SET
again. The script stops bluetoothd, changes the stored key's `Authenticated=4` (debug) to `3`
(authenticated P-256) and starts bluetoothd. The key bytes are unchanged.

Rerun `pair` and the script only if the scale loses the pairing, for example after a battery
change. The service then sends a desktop notification instead of failing silently.

## Commands

| Command | Does |
| --- | --- |
| `run` | The service: collect every 20 minutes and sync |
| `pair` | Pair the laptop (press SET), then print the relabel command |
| `register --address ADDR` | Register a user slot for this laptop |
| `sync` | Send logged days Lifestats has not acknowledged |
| `status` | Show the slot and the latest readings |

`LIFESTATS_URL` overrides `http://localhost:3000`.

## Development

```sh
uv run pytest
```

The protocol tests use packets captured from the scale. Logs: `journalctl --user -u lifestats-beurer-sync`.
