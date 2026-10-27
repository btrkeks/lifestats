"""Send each day's first weigh-in to Lifestats' Beurer integration endpoint."""

import json
import os
import urllib.error
import urllib.request
from datetime import date

from .protocol import Reading
from . import store

LIFESTATS_URL = os.environ.get("LIFESTATS_URL", "http://localhost:3000")


def day_payloads(readings: list[Reading], user_index: int) -> dict[date, dict]:
    """The body of `PUT /integrations/beurer/days/{day}` for every day with a weigh-in by this user.

    A day's value is its first weigh-in: morning weight is the comparable one. Body composition
    fields are left out when that weigh-in had none (shoes on), so Lifestats keeps what it has.
    """
    firsts: dict[date, Reading] = {}
    for reading in readings:
        if reading.user_index != user_index or reading.timestamp is None or not reading.weight_kg:
            continue
        day = reading.timestamp.date()
        if day not in firsts or reading.timestamp < firsts[day].timestamp:
            firsts[day] = reading

    def permille(percent: float | None) -> int | None:
        return None if percent is None else max(0, min(1000, round(percent * 10)))

    payloads = {}
    for day, reading in firsts.items():
        payload = {"weight_grams": round(reading.weight_kg * 1000)}
        for key, percent in [
            ("body_fat_permille", reading.body_fat_pct),
            ("muscle_permille", reading.muscle_pct),
            ("body_water_permille", reading.body_water_pct),
        ]:
            if (value := permille(percent)) is not None:
                payload[key] = value
        payloads[day] = payload
    return payloads


def put_day(day: date, payload: dict, base_url: str = LIFESTATS_URL) -> None:
    request = urllib.request.Request(
        f"{base_url}/integrations/beurer/days/{day.isoformat()}",
        data=json.dumps(payload).encode(),
        headers={"Content-Type": "application/json"},
        method="PUT",
    )
    with urllib.request.urlopen(request, timeout=10) as response:
        if response.status != 204:
            raise RuntimeError(f"Lifestats answered {response.status} for {day}")


def sync_pending(user_index: int, log, base_url: str = LIFESTATS_URL) -> int:
    """PUT every day whose payload Lifestats has not acknowledged yet. Returns how many were sent."""
    synced = store.load_synced()
    sent = 0
    for day, payload in sorted(day_payloads(store.load_readings(), user_index).items()):
        if synced.get(day.isoformat()) == payload:
            continue
        try:
            put_day(day, payload, base_url)
        except urllib.error.HTTPError as error:
            log(f"Lifestats rejected {day}: {error.code} {error.read().decode(errors='replace')}")
            continue
        except (urllib.error.URLError, OSError, RuntimeError) as error:
            log(f"Lifestats unreachable, will retry: {error}")
            break
        synced[day.isoformat()] = payload
        store.save_synced(synced)
        sent += 1
        log(f"sent {day} to Lifestats: {payload}")
    return sent
