from datetime import datetime, timedelta
from threading import Timer
from typing import Optional

from aqt import mw
from aqt.utils import tooltip
from anki.decks import DeckId

from .anki_utils import get_current_deck_id
from .config import get_statistic_id_for_deck
from .statistic_logging import log_time_until_now


class TimerManager:
    def __init__(self, timeout_callback):
        self.timer: Optional[Timer] = None
        self.timeout_callback = timeout_callback

    def schedule_timer(self, interval: timedelta):
        self.cancel_timer()

        interval_secs = interval.total_seconds()
        if interval_secs > 0:
            self.timer = Timer(interval_secs, self.timeout_callback)
            self.timer.start()
        elif interval_secs == 0:
            self.timeout_callback()
        else:
            print("ERROR: Negative timeout")

    def cancel_timer(self):
        if self.timer is not None:
            self.timer.cancel()
            self.timer = None


class AnkiLogger:
    """Tracks and logs Anki study session durations.

    Manages both user idle detection and periodic logging of study time frames.
    Times out after a period of inactivity and periodically flushes logged time
    to the database.
    """

    def __init__(
            self,
            idle_timeout_seconds: int = 60,
            flush_interval_seconds: int = 120
    ) -> None:
        # Configuration
        self.idle_timeout: timedelta = timedelta(seconds=idle_timeout_seconds)
        self.flush_interval: timedelta = timedelta(seconds=flush_interval_seconds)

        # State tracking
        self.is_running: bool = False
        self.deck_id: Optional[DeckId] = None

        # Timing tracking
        self.start_time: Optional[datetime] = None
        self.last_flush: Optional[datetime] = None
        self.last_ping: Optional[datetime] = None

        # Timer management
        self.timer_manager = TimerManager(self._on_timeout)

    def start_running(self) -> None:
        current_time = datetime.now()

        self.is_running = True
        self.start_time = current_time
        self.last_ping = current_time
        self.last_flush = current_time
        self.deck_id = get_current_deck_id()

        statistic_id = get_statistic_id_for_deck(self.deck_id)
        tooltip(f"Start logging for statistic id {statistic_id}", period=3000)

        self._schedule_next_timeout()

    def stop_running(self) -> None:
        if not self.is_running or self.start_time is None:
            return

        tooltip("Stop logging", period=3000)

        self.timer_manager.cancel_timer()
        self._log_time_frame()

        # Reset state
        self.is_running = False
        self.start_time = None
        self.last_ping = None
        self.last_flush = None
        self.deck_id = None

    def ping(self):
        if not self.is_running:
            current_deck_id = get_current_deck_id()
            if get_statistic_id_for_deck(current_deck_id) is not None:
                self.start_running()
        else:
            self.last_ping = datetime.now()
            self._schedule_next_timeout()

    def _flush_time_frame(self):
        self._log_time_frame()
        self.last_flush = datetime.now()

    def _log_time_frame(self):
        statistic_id = get_statistic_id_for_deck(self.deck_id)
        if statistic_id is not None:
            log_time_until_now(statistic_id, self.start_time)

    def _is_idle(self) -> bool:
        """Check if session should timeout due to user inactivity."""
        if self.last_ping is None:
            return True

        time_since_last_ping = datetime.now() - self.last_ping
        return time_since_last_ping >= self.idle_timeout

    def _on_timeout(self):
        if self._is_idle():
            mw.taskman.run_on_main(self.stop_running)
        else:
            self._flush_time_frame()
            self._schedule_next_timeout()

    def calculate_next_timeout_interval(self) -> timedelta:
        if self.last_ping is None or self.last_flush is None:
            raise RuntimeError("last_ping or last_flush must be set")

        next_idle_check = self.last_ping + self.idle_timeout
        next_flush = self.last_flush + self.flush_interval
        next_timeout = min(next_idle_check, next_flush)
        return next_timeout - datetime.now()

    def _schedule_next_timeout(self):
        interval = self.calculate_next_timeout_interval()
        self.timer_manager.schedule_timer(interval)
