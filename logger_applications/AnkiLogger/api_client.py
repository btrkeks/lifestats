"""API client for communicating with the lifestats-web-api.

This module encapsulates all HTTP communication with the web API,
following the Adapter pattern to separate communication details
from application logic.
"""

from concurrent.futures import ThreadPoolExecutor
from datetime import date
from typing import Optional
import sys

try:
    import requests
except ImportError:
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
    def make_request():
        if requests is None:
            error_msg = "requests library not available - cannot log to API"
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

            print(f"Logging to API: POST {url} with payload {payload}")

            response = requests.post(url, json=payload, timeout=10)
            response.raise_for_status()

            print(f"Successfully logged to API (status {response.status_code})")

        except requests.exceptions.ConnectionError as e:
            error_msg = f"Failed to connect to API at {base_url}: {e}"
            print(error_msg, file=sys.stderr)
        except requests.exceptions.Timeout as e:
            error_msg = f"API request timed out: {e}"
            print(error_msg, file=sys.stderr)
        except requests.exceptions.HTTPError as e:
            error_msg = (
                f"API request failed with status {e.response.status_code}: "
                f"{e.response.text}"
            )
            print(error_msg, file=sys.stderr)
        except Exception as e:
            error_msg = f"Unexpected error logging to API: {e}"
            print(error_msg, file=sys.stderr)

    executor.submit(make_request)
