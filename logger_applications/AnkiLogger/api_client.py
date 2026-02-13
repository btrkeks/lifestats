"""API client for communicating with the lifestats-web-api.

This module encapsulates all HTTP communication with the web API,
following the Adapter pattern to separate communication details
from application logic.
"""

from concurrent.futures import ThreadPoolExecutor
from datetime import date
from typing import Optional
from dataclasses import dataclass
import sys
import os

# Add vendor directory to path for bundled dependencies
vendor_path = os.path.join(os.path.dirname(__file__), 'vendor')
if os.path.exists(vendor_path) and vendor_path not in sys.path:
    sys.path.insert(0, vendor_path)

from .logger_config import get_logger

logger = get_logger(__name__)
logger.info(f"Vendor path: {vendor_path}")
logger.info(f"Vendor path exists: {os.path.exists(vendor_path)}")

try:
    import requests
    logger.info(f"requests library successfully imported, version: {requests.__version__}")
except ImportError as e:
    logger.error(f"Failed to import requests library: {e}", exc_info=True)
    requests = None

# Shared thread pool for async operations
executor = ThreadPoolExecutor(max_workers=5)


# Define a structured data class for the bundle response
@dataclass
class StatisticBundle:
    """Bundle of statistic IDs for a deck."""
    time_spent_id: int
    disruptions_id: int
    backlog_id: int
    finished_time_id: int  # Per-deck statistic (when this deck's cards are all done)
    cards_studied_id: int  # Number of unique cards studied today


# Define custom exceptions for clear error handling
class ApiError(Exception):
    """Base class for API-related errors."""
    pass


class ApiConnectionError(ApiError):
    """Raised for network-level errors (e.g., DNS failure, connection refused)."""
    pass


class ApiHttpError(ApiError):
    """Raised for non-2xx HTTP status codes."""
    pass


class ApiValidationError(ApiError):
    """Raised when the API response payload fails validation."""
    pass


class ApiClient:
    """Stateless adapter for the lifestats web API.

    Handles all HTTP requests and responses, completely decoupled from
    the plugin's internal state.
    """

    def __init__(self, base_url: str, timeout: int = 10):
        self.base_url = base_url.rstrip('/')
        self.timeout = timeout
        self.session = requests.Session() if requests else None

    def provision_anki_bundle(self, deck_id: int, deck_name: str) -> Optional[StatisticBundle]:
        """Provisions or retrieves the statistic bundle for a given deck.

        This is idempotent - calling it multiple times with the same context
        will return the same bundle.

        The backend accepts a flattened HashMap of context. We send both
        deck_id (stable identifier) and deck_name (human-readable context).

        Args:
            deck_id: The Anki deck ID (did) - stable identifier
            deck_name: The deck name for human-readable context

        Returns:
            A StatisticBundle on success, or None on failure
        """
        if self.session is None:
            logger.error("requests library not available")
            raise ApiConnectionError("requests library not available")

        url = f"{self.base_url}/integrations/anki"
        # Send both deck_id and deck_name for maximum robustness
        payload = {
            "deck_id": str(deck_id),
            "deck_name": deck_name
        }

        try:
            logger.info(f"Provisioning bundle for deck '{deck_name}' (did={deck_id})")
            response = self.session.post(url, json=payload, timeout=self.timeout)
            response.raise_for_status()

            data = response.json()

            # Validate that response contains all required keys
            required_keys = {'time_spent_id', 'disruptions_id', 'backlog_id', 'finished_time_id', 'cards_studied_id'}
            missing_keys = required_keys - set(data.keys())
            if missing_keys:
                # Log the problematic payload for easy debugging
                logger.error(f"API response missing required keys: {missing_keys}. Payload: {data}")
                raise ApiValidationError(f"API response is missing required keys: {', '.join(missing_keys)}")

            bundle = StatisticBundle(
                time_spent_id=data['time_spent_id'],
                disruptions_id=data['disruptions_id'],
                backlog_id=data['backlog_id'],
                finished_time_id=data['finished_time_id'],
                cards_studied_id=data['cards_studied_id'],
            )
            logger.info(f"Successfully provisioned and validated bundle: {bundle}")
            return bundle

        except requests.exceptions.ConnectionError as e:
            raise ApiConnectionError(f"Could not connect to API at {url}: {e}") from e
        except requests.exceptions.Timeout as e:
            raise ApiConnectionError(f"API request timed out for {url}: {e}") from e
        except requests.exceptions.HTTPError as e:
            raise ApiHttpError(
                f"API returned status {e.response.status_code} for {url}: {e.response.text}"
            ) from e
        except ApiValidationError:
            # Re-raise to be caught by the SessionTracker
            raise
        except (KeyError, TypeError) as e:
            # This block now becomes a fallback for unexpected structural issues
            raise ApiValidationError(f"Failed to parse API response for {url}: {e}") from e

    def log_statistic_value(
        self,
        statistic_id: int,
        date: date,
        value: str,
        update_policy: Optional[str] = None
    ) -> None:
        """Log a statistic value to the web API synchronously.

        Args:
            statistic_id: The ID of the statistic to log to
            date: The date to log the value for
            value: The value to log (format depends on statistic type)
            update_policy: Optional update policy (e.g., "sum", "overwrite")
        """
        if self.session is None:
            logger.error("requests library not available")
            raise ApiConnectionError("requests library not available")

        date_str = date.isoformat()
        url = f"{self.base_url}/statistics/{statistic_id}/log/{date_str}"

        payload = {"value": value}
        if update_policy:
            payload["update_policy"] = update_policy

        try:
            response = self.session.post(url, json=payload, timeout=self.timeout)
            response.raise_for_status()
            logger.info(f"Successfully logged value to statistic {statistic_id}")

        except requests.exceptions.ConnectionError as e:
            raise ApiConnectionError(f"Could not connect to API at {url}: {e}") from e
        except requests.exceptions.Timeout as e:
            raise ApiConnectionError(f"API request timed out for {url}: {e}") from e
        except requests.exceptions.HTTPError as e:
            raise ApiHttpError(
                f"API returned status {e.response.status_code} for {url}: {e.response.text}"
            ) from e

    def log_time_frame(
        self,
        statistic_id: int,
        date: date,
        start_time: str,
        end_time: str,
        update_policy: Optional[str] = None
    ) -> None:
        """Log a time frame to a TimeSpans statistic synchronously.

        Args:
            statistic_id: The ID of the statistic to log to
            date: The date to log the time frame for
            start_time: Start time in HH:MM or HH:MM:SS format
            end_time: End time in HH:MM or HH:MM:SS format
            update_policy: Optional update policy (e.g., "append", "updateendvalue")
        """
        if self.session is None:
            logger.error("requests library not available")
            raise ApiConnectionError("requests library not available")

        date_str = date.isoformat()
        url = f"{self.base_url}/statistics/{statistic_id}/log_time_frame/{date_str}"

        payload = {
            "start_time": start_time,
            "end_time": end_time
        }
        if update_policy:
            payload["update_policy"] = update_policy

        try:
            response = self.session.post(url, json=payload, timeout=self.timeout)
            response.raise_for_status()
            logger.info(f"Successfully logged time frame to statistic {statistic_id}")

        except requests.exceptions.ConnectionError as e:
            raise ApiConnectionError(f"Could not connect to API at {url}: {e}") from e
        except requests.exceptions.Timeout as e:
            raise ApiConnectionError(f"API request timed out for {url}: {e}") from e
        except requests.exceptions.HTTPError as e:
            raise ApiHttpError(
                f"API returned status {e.response.status_code} for {url}: {e.response.text}"
            ) from e

    def log_statistic_value_async(
        self,
        statistic_id: int,
        date: date,
        value: str,
        update_policy: Optional[str] = None
    ) -> None:
        """Log a statistic value to the web API asynchronously.

        This function executes the HTTP request in a background thread to keep
        the Anki UI responsive.
        """
        def make_request():
            try:
                self.log_statistic_value(statistic_id, date, value, update_policy)
            except ApiError as e:
                logger.error(f"Failed to log statistic value: {e}", exc_info=True)

        executor.submit(make_request)

    def log_time_frame_async(
        self,
        statistic_id: int,
        date: date,
        start_time: str,
        end_time: str,
        update_policy: Optional[str] = None
    ) -> None:
        """Log a time frame to the web API asynchronously.

        This function executes the HTTP request in a background thread to keep
        the Anki UI responsive.
        """
        def make_request():
            try:
                self.log_time_frame(statistic_id, date, start_time, end_time, update_policy)
            except ApiError as e:
                logger.error(f"Failed to log time frame: {e}", exc_info=True)

        executor.submit(make_request)


