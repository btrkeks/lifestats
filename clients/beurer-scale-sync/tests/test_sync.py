from datetime import datetime

from beurer_scale_sync import store
from beurer_scale_sync.lifestats import day_payloads
from beurer_scale_sync.protocol import Reading


def reading(when: str, weight: float, user: int = 2, **body) -> Reading:
    return Reading(user, datetime.fromisoformat(when), weight_kg=weight, **body)


def test_first_weigh_in_of_the_day_wins_regardless_of_arrival_order():
    payloads = day_payloads(
        [
            reading("2026-10-27T11:11:47", 67.5, body_fat_pct=15.6, muscle_pct=42.6, water_mass_kg=41.1),
            reading("2026-10-27T07:02:10", 67.1, body_fat_pct=16.2, muscle_pct=42.0, water_mass_kg=40.6),
            reading("2026-10-28T07:15:00", 66.9),
        ],
        user_index=2,
    )
    assert payloads[datetime(2026, 10, 27).date()] == {
        "weight_grams": 67100,
        "body_fat_permille": 162,
        "muscle_permille": 420,
        "body_water_permille": 605,
    }
    # No body composition (shoes on): only weight is sent, Lifestats keeps the rest.
    assert payloads[datetime(2026, 10, 28).date()] == {"weight_grams": 66900}


def test_other_users_and_undated_readings_are_ignored():
    payloads = day_payloads(
        [reading("2026-10-27T06:00:00", 80.0, user=1), Reading(2, None, weight_kg=67.0)], user_index=2
    )
    assert payloads == {}


def test_append_readings_skips_readings_the_scale_resends(tmp_path):
    log = tmp_path / "readings.jsonl"
    first = reading("2026-10-27T10:57:42", 67.6, body_fat_pct=16.0)
    assert store.append_readings([first], log) == [first]
    second = reading("2026-10-27T11:11:47", 67.5)
    assert store.append_readings([first, second], log) == [second]
    assert store.load_readings(log) == [first, second]


def test_credentials_round_trip_and_are_private(tmp_path):
    path = tmp_path / "state.json"
    store.save_credentials(store.Credentials("D8:0B:CB:5F:A5:2D", 2, 16), path)
    assert store.load_credentials(path) == store.Credentials("D8:0B:CB:5F:A5:2D", 2, 16)
    assert path.stat().st_mode & 0o777 == 0o600
