from datetime import date, datetime

from beurer_scale_sync.protocol import Reading
from beurer_scale_sync.schedule import Window, has_reading_on

WINDOW = Window()


def at(text: str) -> datetime:
    return datetime.fromisoformat(f"2026-10-28T{text}")


def test_waits_for_the_morning_start():
    assert not WINDOW.should_collect(at("05:50"), have_todays_reading=False)
    assert WINDOW.next_check(at("05:50"), have_todays_reading=False) == at("06:00")


def test_collects_from_the_start_and_retries_until_the_reading_arrives():
    assert WINDOW.should_collect(at("06:00"), have_todays_reading=False)
    assert WINDOW.next_check(at("06:00:10"), have_todays_reading=False) == at("06:15:10")


def test_stops_for_the_day_once_the_reading_is_in():
    assert not WINDOW.should_collect(at("06:15"), have_todays_reading=True)
    assert WINDOW.next_check(at("06:00:10"), have_todays_reading=True) == datetime(2026, 10, 29, 6, 0)


def test_a_laptop_waking_late_collects_at_once():
    assert WINDOW.should_collect(at("09:41"), have_todays_reading=False)


def test_gives_up_at_the_end_of_the_day():
    assert WINDOW.next_check(at("22:50"), have_todays_reading=False) == at("23:00")
    assert not WINDOW.should_collect(at("23:00"), have_todays_reading=False)
    assert WINDOW.next_check(at("23:00"), have_todays_reading=False) == datetime(2026, 10, 29, 6, 0)


def test_only_this_slots_readings_from_that_day_count():
    readings = [
        Reading(2, datetime(2026, 10, 27, 5, 50), weight_kg=67.5),
        Reading(1, datetime(2026, 10, 28, 5, 50), weight_kg=80.0),
        Reading(2, None, weight_kg=67.4),
    ]
    assert has_reading_on(readings, date(2026, 10, 27), user_index=2)
    assert not has_reading_on(readings, date(2026, 10, 28), user_index=2)
