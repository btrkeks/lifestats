"""When the service talks to the scale: once a morning, until that day's weigh-in is in.

The BF500 stores weigh-ins taken while nobody is connected, and its advertisements look the same
before and after one, so the service cannot react to a weigh-in. Instead it starts collecting at
a fixed time each morning, retries until a reading from that day arrives, and then leaves the
scale alone until the next morning.
"""

from dataclasses import dataclass
from datetime import date, datetime, time, timedelta

from .protocol import Reading


@dataclass(frozen=True)
class Window:
    start: time = time(6, 0)
    end: time = time(23, 0)
    retry: timedelta = timedelta(minutes=15)

    def should_collect(self, now: datetime, have_todays_reading: bool) -> bool:
        return not have_todays_reading and self.start <= now.time() < self.end

    def next_check(self, now: datetime, have_todays_reading: bool) -> datetime:
        todays_start = datetime.combine(now.date(), self.start)
        if now < todays_start:
            return todays_start
        if have_todays_reading or now.time() >= self.end:
            return todays_start + timedelta(days=1)
        return min(now + self.retry, datetime.combine(now.date(), self.end))


def has_reading_on(readings: list[Reading], day: date, user_index: int) -> bool:
    return any(
        reading.user_index == user_index and reading.timestamp is not None and reading.timestamp.date() == day
        for reading in readings
    )
