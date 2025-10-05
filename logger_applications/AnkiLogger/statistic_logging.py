from datetime import datetime

from . import api_client


def log_current_time(statistic_id: int):
    """Log the current time to a PointInTime statistic via the web API.

    Args:
        statistic_id: The ID of the statistic to log to
    """
    print(f"Attempting to log current time for statistic_id: {statistic_id}")
    now = datetime.now()
    date = now.date()
    value_str = now.strftime("%H:%M:%S")

    api_client.log_statistic_value_async(
        statistic_id=statistic_id,
        date=date,
        value=value_str,
        update_policy=None
    )


def log_time_until_now(statistic_id: int, start: datetime):
    """Log a time span from start until now to a TimeSpans statistic via the web API.

    Args:
        statistic_id: The ID of the statistic to log to
        start: The start time of the time span
    """
    end = datetime.now()
    date = end.date()
    value_str = f'{start.strftime("%H:%M:%S")}-{end.strftime("%H:%M:%S")}'

    api_client.log_statistic_value_async(
        statistic_id=statistic_id,
        date=date,
        value=value_str,
        update_policy="updateendvalue"
    )
