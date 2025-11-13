"""Session tracker for Anki study sessions.

This module contains the core business logic for tracking study sessions,
managing timers, and orchestrating provisioning and logging.
"""

from datetime import datetime, timedelta
from threading import Timer
from typing import Optional

from anki.decks import DeckId
from aqt import mw
from aqt.utils import tooltip

from .anki_utils import get_current_deck_id, get_deck_name, get_total_backlog_count
from .api_client import ApiClient, ApiError, StatisticBundle
from .config_manager import ConfigManager
from .logger_config import get_logger

logger = get_logger(__name__)


class TimerManager:
    """Manages timeout timers for session tracking."""

    def __init__(self, timeout_callback):
        self.timer: Optional[Timer] = None
        self.timeout_callback = timeout_callback

    def schedule_timer(self, interval: timedelta):
        """Schedule a timer to fire after the given interval."""
        self.cancel_timer()

        interval_secs = interval.total_seconds()
        if interval_secs > 0:
            self.timer = Timer(interval_secs, self.timeout_callback)
            self.timer.start()
        elif interval_secs == 0:
            self.timeout_callback()
        else:
            logger.error(f"Negative timeout interval: {interval_secs}")

    def cancel_timer(self):
        """Cancel any active timer."""
        if self.timer is not None:
            self.timer.cancel()
            self.timer = None


class SessionTracker:
    """Tracks and logs Anki study session durations.

    Manages user idle detection, periodic logging of study time frames,
    and automatic deck provisioning. This is the application core that
    orchestrates the ApiClient and ConfigManager.
    """

    def __init__(
        self,
        api_client: ApiClient,
        config_manager: ConfigManager,
        idle_timeout_seconds: int = 60,
        flush_interval_seconds: int = 120,
    ) -> None:
        # Dependencies
        self.api_client = api_client
        self.config_manager = config_manager

        # Configuration
        self.idle_timeout: timedelta = timedelta(seconds=idle_timeout_seconds)
        self.flush_interval: timedelta = timedelta(seconds=flush_interval_seconds)

        # State tracking
        self.is_running: bool = False
        self.deck_id: Optional[DeckId] = None
        self.deck_name: Optional[str] = None
        self.active_bundle: Optional[StatisticBundle] = None

        # Timing tracking
        self.start_time: Optional[datetime] = None
        self.last_flush: Optional[datetime] = None
        self.last_ping: Optional[datetime] = None

        # Timer management
        self.timer_manager = TimerManager(self._on_timeout)

        # Track which decks we've shown error tooltips for
        self._notified_errors: set = set()

    def _provision_and_start_session(self, deck_id: DeckId) -> bool:
        """Ensures a deck is configured before starting a session.

        This method orchestrates the entire provisioning workflow:
        1. Check cache for existing bundle
        2. If not cached, call API to provision
        3. If successful, save to cache
        4. Start the session with the bundle

        Returns:
            True if session started successfully, False otherwise
        """
        deck_name = get_deck_name(deck_id)
        if not deck_name:
            logger.warning(f"Could not get deck name for deck_id={deck_id}")
            return False

        logger.info(
            f"Attempting to start session for deck '{deck_name}' (did={deck_id})"
        )

        # Step 1: Check the cache via the simple ConfigManager
        bundle = self.config_manager.get_deck_bundle(deck_id)

        # Step 2: If not in cache, call the API to provision
        if not bundle:
            # Don't retry a failed deck in the same session
            if deck_id in self._notified_errors:
                logger.debug(
                    f"Deck {deck_id} previously failed to provision this session"
                )
                return False

            try:
                tooltip(f"Configuring '{deck_name}' with lifestats server...")
                logger.info(
                    f"Provisioning bundle for deck '{deck_name}' (did={deck_id})"
                )

                bundle = self.api_client.provision_anki_bundle(deck_id, deck_name)

                if not bundle:
                    raise ApiError("Provisioning returned no bundle.")

                # Step 3: If API call is successful, save to the cache
                self.config_manager.save_deck_bundle(deck_id, bundle)
                tooltip(f"'{deck_name}' configured successfully!", period=3000)
                logger.info(f"Successfully provisioned and cached bundle: {bundle}")

            except ApiError as e:
                # Graceful failure
                error_msg = f"Failed to configure '{deck_name}': Invalid response from server. Check logs."
                logger.error(
                    f"Failed to provision deck '{deck_name}': {e}", exc_info=True
                )
                tooltip(error_msg, period=5000)
                self._notified_errors.add(deck_id)
                return False

        # If we reach here, we have a valid bundle - start the session
        current_time = datetime.now()
        self.is_running = True
        self.deck_id = deck_id
        self.deck_name = deck_name
        self.active_bundle = bundle
        self.start_time = current_time
        self.last_ping = current_time
        self.last_flush = current_time

        logger.info(f"=== STARTING LOGGING SESSION ===")
        logger.info(f"Deck ID: {deck_id}")
        logger.info(f"Deck Name: '{deck_name}'")
        logger.info(f"Bundle: {bundle}")

        # Fulfill new functional requirements
        self._log_disruption()
        self._log_backlog()

        # Start timers
        self._schedule_next_timeout()

        # Show success tooltip (brief, only if we just provisioned)
        if deck_id not in self._notified_errors:
            tooltip(f"Tracking deck: {deck_name}", period=2000)

        return True

    def start_running(self) -> None:
        """Legacy method for backward compatibility."""
        current_deck_id = get_current_deck_id()
        if current_deck_id:
            self._provision_and_start_session(current_deck_id)

    def stop_running(self) -> None:
        """Stops the current session and logs final time frame."""
        if not self.is_running or self.start_time is None:
            return

        logger.info("Stopping logging session")
        tooltip("Stopped tracking", period=2000)

        self.timer_manager.cancel_timer()
        self._log_time_frame()
        self._log_backlog()

        # Reset state
        self.is_running = False
        self.start_time = None
        self.last_ping = None
        self.last_flush = None
        self.deck_id = None
        self.deck_name = None
        self.active_bundle = None

    def ping(self):
        """Called on user activity. Starts a new session if required."""
        if not self.is_running:
            current_deck_id = get_current_deck_id()
            if current_deck_id:
                logger.debug(
                    f"ping() called while not running for deck_id={current_deck_id}"
                )
                self._provision_and_start_session(current_deck_id)
        else:
            logger.debug(f"ping() called while running, updating last_ping time")
            self.last_ping = datetime.now()
            self._schedule_next_timeout()

    def _flush_time_frame(self):
        """Flushes the current time frame and updates last_flush."""
        self._log_time_frame()
        self.last_flush = datetime.now()

    def _log_time_frame(self):
        """Logs accumulated study time to the time_spent statistic."""
        if not self.active_bundle or not self.start_time:
            return

        start_time_str = self.start_time.strftime("%H:%M:%S")
        end_time_str = datetime.now().strftime("%H:%M:%S")
        date = datetime.now().date()

        logger.info(
            f"Logging time frame: {start_time_str} -> {end_time_str} "
            f"to statistic {self.active_bundle.time_spent_id}"
        )

        try:
            self.api_client.log_time_frame_async(
                statistic_id=self.active_bundle.time_spent_id,
                date=date,
                start_time=start_time_str,
                end_time=end_time_str,
                update_policy="updateendvalue",
            )
        except Exception as e:
            logger.error(f"Failed to log time frame: {e}", exc_info=True)

    def _log_disruption(self):
        """Logs a 'disruption' event (start of a new session).

        Increments the disruption counter by 1 using sum policy.
        """
        if not self.active_bundle:
            return

        date = datetime.now().date()
        logger.info(
            f"Logging disruption to statistic {self.active_bundle.disruptions_id}"
        )

        try:
            self.api_client.log_statistic_value_async(
                statistic_id=self.active_bundle.disruptions_id,
                date=date,
                value="1",
                update_policy="sum",
            )
        except Exception as e:
            logger.error(f"Failed to log disruption: {e}", exc_info=True)

    def _log_backlog(self):
        """Logs the current total backlog count (New + Learning + Review).

        Overwrites the current backlog value with latest count.
        """
        if not self.active_bundle or not self.deck_id:
            return

        backlog_count = get_total_backlog_count(self.deck_id)
        date = datetime.now().date()

        logger.info(
            f"Logging total backlog count ({backlog_count}) "
            f"to statistic {self.active_bundle.backlog_id}"
        )

        try:
            self.api_client.log_statistic_value_async(
                statistic_id=self.active_bundle.backlog_id,
                date=date,
                value=str(backlog_count),
                update_policy="overwrite",
            )
        except Exception as e:
            logger.error(f"Failed to log backlog: {e}", exc_info=True)

    def log_finished_time(self):
        """Logs the current time as the 'finished time' for the current deck.

        This is called when all cards in the current deck are completed for the day.
        Uses 'overwrite' policy so that multiple finishes in one day record the
        last finish time.
        """
        logger.debug("Logging finished time")
        if not self.active_bundle:
            logger.warning("Cannot log finished time: no active bundle")
            return

        current_time = datetime.now()
        time_str = current_time.strftime("%H:%M:%S")
        date = current_time.date()

        logger.info(
            f"Logging finished time ({time_str}) "
            f"to statistic {self.active_bundle.finished_time_id}"
        )

        try:
            self.api_client.log_statistic_value_async(
                statistic_id=self.active_bundle.finished_time_id,
                date=date,
                value=time_str,
                update_policy="ignore",
            )
            logger.info(f"Successfully logged finished time: {time_str}")
        except Exception as e:
            logger.error(f"Failed to log finished time: {e}", exc_info=True)

    def _is_idle(self) -> bool:
        """Check if session should timeout due to user inactivity."""
        if self.last_ping is None:
            return True

        time_since_last_ping = datetime.now() - self.last_ping
        return time_since_last_ping >= self.idle_timeout

    def _on_timeout(self):
        """Called when a timer fires - either idle timeout or flush interval."""
        if self._is_idle():
            logger.info("Idle timeout reached, stopping session")
            mw.taskman.run_on_main(self.stop_running)
        else:
            logger.debug("Flush interval reached, flushing time frame")
            self._flush_time_frame()
            self._schedule_next_timeout()

    def calculate_next_timeout_interval(self) -> timedelta:
        """Calculate when the next timeout should fire."""
        if self.last_ping is None or self.last_flush is None:
            raise RuntimeError("last_ping or last_flush must be set")

        next_idle_check = self.last_ping + self.idle_timeout
        next_flush = self.last_flush + self.flush_interval
        next_timeout = min(next_idle_check, next_flush)
        return next_timeout - datetime.now()

    def _schedule_next_timeout(self):
        """Schedule the next timeout based on idle and flush intervals."""
        interval = self.calculate_next_timeout_interval()
        self.timer_manager.schedule_timer(interval)
