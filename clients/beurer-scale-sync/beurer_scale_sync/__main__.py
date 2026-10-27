"""Command line: `run` is the background service; `pair` and `register` are one-time setup."""

import argparse
import asyncio
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

from bleak import BleakClient, BleakScanner

from . import agent, lifestats, store
from .schedule import Window, has_reading_on
from .session import collect_readings, register_slot

FIND_TIMEOUT = 60
FAILURES_BEFORE_WARNING = 3
NOTIFY_INTERVAL = 3600


def log(*parts) -> None:
    print(*parts, flush=True)


class Notifier:
    """Desktop notifications, at most one per hour per topic, so a broken pairing doesn't spam."""

    def __init__(self):
        self.last: dict[str, float] = {}

    def __call__(self, topic: str, message: str) -> None:
        log(f"{topic}: {message}")
        now = time.monotonic()
        if now - self.last.get(topic, -NOTIFY_INTERVAL) < NOTIFY_INTERVAL:
            return
        self.last[topic] = now
        try:
            subprocess.run(["notify-send", "-a", "Lifestats", "Beurer scale", message], check=False)
        except FileNotFoundError:
            pass


async def sleep_until(deadline: float) -> None:
    """Sleep on the wall clock, so a laptop waking from suspend collects within a minute."""
    while time.time() < deadline:
        await asyncio.sleep(min(60, max(1, deadline - time.time())))


async def collect_once(bus, credentials: store.Credentials, notify: Notifier, failures: int) -> int:
    """One collection attempt. Returns the updated count of consecutive failures."""
    await agent.ensure_agent_registered(bus)
    device = await BleakScanner.find_device_by_address(credentials.address, timeout=FIND_TIMEOUT)
    if device is None:
        log("scale not in range")
        return failures
    try:
        readings = await collect_readings(device, credentials, log)
    except Exception as error:  # Bluetooth errors are varied and all mean "try again next time".
        failures += 1
        log(f"collection failed ({failures} in a row): {error!r}")
        if failures >= FAILURES_BEFORE_WARNING:
            notify("failing", f"Collecting from the scale failed {failures} times. If it keeps happening, run `beurer-scale-sync pair`.")
        return failures
    new = store.append_readings(readings)
    log(f"collected {len(readings)} reading(s), {len(new)} new")
    for reading in new:
        log(f"new reading {reading.timestamp}: {reading.weight_kg} kg, body fat {reading.body_fat_pct} %")
    return 0


async def run(args) -> None:
    credentials = store.load_credentials()
    window = Window(start=args.start, end=args.end)
    notify = Notifier()
    pairing_message = "The scale asked to pair again. Run `beurer-scale-sync pair` and press SET on the scale."
    bus = await agent.register_agent(
        credentials.address, confirm=False, on_pairing_request=lambda: notify("pairing", pairing_message), log=log
    )
    log(
        f"collecting from scale {credentials.address}, slot {credentials.user_index}, "
        f"daily from {window.start:%H:%M} until that day's reading arrives (last try {window.end:%H:%M})"
    )
    failures = 0
    while True:
        lifestats.sync_pending(credentials.user_index, log)
        today = datetime.now()
        if window.should_collect(today, has_reading_on(store.load_readings(), today.date(), credentials.user_index)):
            failures = await collect_once(bus, credentials, notify, failures)
            lifestats.sync_pending(credentials.user_index, log)
        now = datetime.now()
        next_check = window.next_check(now, has_reading_on(store.load_readings(), now.date(), credentials.user_index))
        log(f"next check {next_check:%Y-%m-%d %H:%M}")
        await sleep_until(next_check.timestamp())


RELABEL_HINT = """
Paired. Linux discards this pairing on the next restart because the BF500 pairs with the
Bluetooth specification's debug key. Keep it by relabelling the stored key once:

    sudo {script} {address}
"""


async def pair(args) -> None:
    address = args.address or store.load_credentials().address
    bus = await agent.register_agent(address, confirm=True, on_pairing_request=lambda: None, log=log)
    await agent.prepare_adapter_for_pairing(bus, address, log)
    log("step on the scale; press SET when it shows PAIR")
    device = await BleakScanner.find_device_by_address(address, timeout=180)
    if device is None:
        sys.exit("the scale did not wake up within 3 minutes")
    try:
        credentials = store.load_credentials()
    except FileNotFoundError:
        credentials = None
    if credentials is not None and credentials.address == address:
        readings = await collect_readings(device, credentials, log, connect_timeout=90)
        store.append_readings(readings)
        log(f"slot {credentials.user_index} accepted the laptop; {len(readings)} reading(s) received")
    else:
        async with BleakClient(device, timeout=90) as client:
            await client.pair()
        log("paired; now run `beurer-scale-sync register`")
    script = Path(__file__).resolve().parent.parent / "relabel-bond.sh"
    print(RELABEL_HINT.format(script=script, address=address))


async def register(args) -> None:
    if store.CREDENTIALS.exists() and not args.force:
        sys.exit(f"{store.CREDENTIALS} already exists; pass --force to register another slot")
    log("step on the scale")
    device = await BleakScanner.find_device_by_address(args.address, timeout=180)
    if device is None:
        sys.exit("the scale did not wake up within 3 minutes")
    credentials = await register_slot(device, args.address, log)
    store.save_credentials(credentials)
    log(f"registered slot {credentials.user_index}; saved to {store.CREDENTIALS}")


async def sync(args) -> None:
    sent = lifestats.sync_pending(store.load_credentials().user_index, log)
    log(f"{sent} day(s) sent")


def status(args) -> None:
    credentials = store.load_credentials()
    readings = [r for r in store.load_readings() if r.user_index == credentials.user_index]
    synced = store.load_synced()
    log(f"scale {credentials.address}, slot {credentials.user_index}")
    log(f"{len(readings)} reading(s) logged in {store.READINGS}; {len(synced)} day(s) acknowledged by Lifestats")
    for reading in readings[-5:]:
        log(f"  {reading.timestamp}: {reading.weight_kg} kg, body fat {reading.body_fat_pct} %, muscle {reading.muscle_pct} %")


def clock_time(text: str):
    return datetime.strptime(text, "%H:%M").time()


def main() -> None:
    parser = argparse.ArgumentParser(prog="beurer-scale-sync", description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    run_parser = commands.add_parser("run", help="collect each morning's weigh-in and sync it (the systemd service)")
    run_parser.add_argument("--start", type=clock_time, default=Window.start, help="first collection each day, HH:MM (default 06:00)")
    run_parser.add_argument("--end", type=clock_time, default=Window.end, help="last retry each day, HH:MM (default 23:00)")
    pair_parser = commands.add_parser("pair", help="pair the laptop with the scale (press SET on the scale)")
    pair_parser.add_argument("--address", help="scale address, defaults to the registered one")
    register_parser = commands.add_parser("register", help="register a user slot for this laptop on the scale")
    register_parser.add_argument("--address", required=True)
    register_parser.add_argument("--force", action="store_true")
    commands.add_parser("sync", help="send logged readings Lifestats has not acknowledged yet")
    commands.add_parser("status", help="show the slot and the latest readings")
    args = parser.parse_args()
    if args.command == "status":
        status(args)
    else:
        asyncio.run({"run": run, "pair": pair, "register": register, "sync": sync}[args.command](args))


if __name__ == "__main__":
    main()
