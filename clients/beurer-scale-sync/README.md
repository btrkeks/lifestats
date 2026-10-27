# Beurer scale sync

Reads a Beurer BF500 bathroom scale over Bluetooth LE and sends each day's first weigh-in to
Lifestats (`PUT /integrations/beurer/days/{day}`): weight, body fat, muscle and body water.
The phone app is not involved. The laptop registers its own user slot on the scale.

Every reading is also appended to `~/.local/share/lifestats-beurer/readings.jsonl`, so nothing
is lost while Lifestats is down. `~/.local/share/lifestats-beurer/synced-days.json` records what
Lifestats acknowledged; delete a day from it to resend that day.

## How a weigh-in reaches Lifestats

1. You step on the scale. It wakes up and advertises for about a minute.
2. The service connects, sets the scale's clock and gives the consent code for its slot.
3. Readings the scale stored while the laptop was away arrive first. If there are none, the service
   asks the scale to measure whoever is standing on it. The display shows APP meanwhile.
4. The service disconnects, logs the readings and sends any day whose first weigh-in changed.

The laptop must be awake and within Bluetooth range. Readings taken while it is not arrive on the
next connection.

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
saved to `~/.config/lifestats-beurer/state.json`. Delete the other user slots on the scale
(for example from the Beurer app), otherwise the scale alternates "U: 1" and "U: 2" when it
cannot tell who is standing on it.

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
| `run` | The service: wait for the scale, collect, sync, repeat |
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
