"""API client for communicating with the lifestats-web-api.

This module encapsulates all HTTP communication with the web API,
following the Adapter pattern to separate communication details
from application logic.
"""

from concurrent.futures import ThreadPoolExecutor
from datetime import date
from typing import Optional
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

from .config import application_config

# Shared thread pool for async operations
executor = ThreadPoolExecutor(max_workers=5)


def log_statistic_value_async(
    statistic_id: int,
    date: date,
    value: str,
    update_policy: Optional[str] = None
) -> None:
    """Log a statistic value to the web API asynchronously.

    This function executes the HTTP request in a background thread to keep
    the Anki UI responsive.

    Args:
        statistic_id: The ID of the statistic to log to
        date: The date to log the value for
        value: The value to log (format depends on statistic type)
        update_policy: Optional update policy (e.g., "updateendvalue")
    """
    logger.info(
        f"log_statistic_value_async called: statistic_id={statistic_id}, "
        f"date={date}, value={value}, update_policy={update_policy}"
    )

    def make_request():
        logger.debug("Starting background request thread")

        if requests is None:
            error_msg = "requests library not available - cannot log to API"
            logger.error(error_msg)
            print(error_msg, file=sys.stderr)
            return

        try:
            base_url = application_config.get_api_base_url()
            date_str = date.isoformat()
            url = f"{base_url}/statistics/{statistic_id}/log/{date_str}"

            # Build payload - only include update_policy if provided
            payload = {"value": value}
            if update_policy:
                payload["update_policy"] = update_policy

            logger.info(f"Making API request: POST {url}")
            logger.debug(f"Request payload: {payload}")
            print(f"Logging to API: POST {url} with payload {payload}")

            response = requests.post(url, json=payload, timeout=10)
            logger.info(f"Received response with status code: {response.status_code}")
            logger.debug(f"Response headers: {dict(response.headers)}")
            logger.debug(f"Response body: {response.text}")

            response.raise_for_status()

            logger.info(f"Successfully logged to API (status {response.status_code})")
            print(f"Successfully logged to API (status {response.status_code})")

        except requests.exceptions.ConnectionError as e:
            error_msg = f"Failed to connect to API at {base_url}: {e}"
            logger.error(error_msg, exc_info=True)
            print(error_msg, file=sys.stderr)
        except requests.exceptions.Timeout as e:
            error_msg = f"API request timed out: {e}"
            logger.error(error_msg, exc_info=True)
            print(error_msg, file=sys.stderr)
        except requests.exceptions.HTTPError as e:
            error_msg = (
                f"API request failed with status {e.response.status_code}: "
                f"{e.response.text}"
            )
            logger.error(error_msg, exc_info=True)
            print(error_msg, file=sys.stderr)
        except Exception as e:
            error_msg = f"Unexpected error logging to API: {e}"
            logger.error(error_msg, exc_info=True)
            print(error_msg, file=sys.stderr)

    logger.debug("Submitting request to thread pool executor")
    executor.submit(make_request)


def log_time_frame_async(
    statistic_id: int,
    date: date,
    start_time: str,
    end_time: str,
    update_policy: Optional[str] = None
) -> None:
    """Log a time frame to a TimeSpans statistic via the web API asynchronously.

    This function uses the type-safe time frame endpoint that accepts
    structured start_time and end_time fields rather than a formatted string.

    Args:
        statistic_id: The ID of the statistic to log to
        date: The date to log the time frame for
        start_time: Start time in HH:MM or HH:MM:SS format
        end_time: End time in HH:MM or HH:MM:SS format
        update_policy: Optional update policy (e.g., "append", "updateendvalue")
    """
    logger.info(
        f"log_time_frame_async called: statistic_id={statistic_id}, "
        f"date={date}, start_time={start_time}, end_time={end_time}, "
        f"update_policy={update_policy}"
    )

    def make_request():
        logger.debug("Starting background request thread for time frame logging")

        if requests is None:
            error_msg = "requests library not available - cannot log to API"
            logger.error(error_msg)
            print(error_msg, file=sys.stderr)
            return

        try:
            base_url = application_config.get_api_base_url()
            date_str = date.isoformat()
            url = f"{base_url}/statistics/{statistic_id}/log_time_frame/{date_str}"

            # Build payload with structured time fields
            payload = {
                "start_time": start_time,
                "end_time": end_time
            }
            if update_policy:
                payload["update_policy"] = update_policy

            logger.info(f"Making API request: POST {url}")
            logger.debug(f"Request payload: {payload}")
            print(f"Logging time frame to API: POST {url} with payload {payload}")

            response = requests.post(url, json=payload, timeout=10)
            logger.info(f"Received response with status code: {response.status_code}")
            logger.debug(f"Response headers: {dict(response.headers)}")
            logger.debug(f"Response body: {response.text}")

            response.raise_for_status()

            logger.info(f"Successfully logged time frame to API (status {response.status_code})")
            print(f"Successfully logged time frame to API (status {response.status_code})")

        except requests.exceptions.ConnectionError as e:
            error_msg = f"Failed to connect to API at {base_url}: {e}"
            logger.error(error_msg, exc_info=True)
            print(error_msg, file=sys.stderr)
        except requests.exceptions.Timeout as e:
            error_msg = f"API request timed out: {e}"
            logger.error(error_msg, exc_info=True)
            print(error_msg, file=sys.stderr)
        except requests.exceptions.HTTPError as e:
            error_msg = (
                f"API request failed with status {e.response.status_code}: "
                f"{e.response.text}"
            )
            logger.error(error_msg, exc_info=True)
            print(error_msg, file=sys.stderr)
        except Exception as e:
            error_msg = f"Unexpected error logging time frame to API: {e}"
            logger.error(error_msg, exc_info=True)
            print(error_msg, file=sys.stderr)

    logger.debug("Submitting time frame request to thread pool executor")
    executor.submit(make_request)
