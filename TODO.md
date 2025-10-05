# TODO: Refactor AnkiLogger to use the Web API

## 🎯 High-Level Goal

The primary goal of this refactor is to decouple the `AnkiLogger` from the `log_statistic` CLI tool and transition it to a proper client of the `lifestats-web-api`. This aligns with our architectural vision of a "Unified logging transport" where all loggers act as independent, API-driven services.

**Current Architecture (Brittle):**
`AnkiLogger (Python) --[subprocess]--> log_statistic (CLI) --[direct DB write]--> SQLite`

**Target Architecture (Decoupled & Robust):**
`AnkiLogger (Python) --[HTTP POST]--> Web API --[LoggingService]--> Domain --> SQLite`

This change centralizes all logging business logic within the Rust application core, ensuring consistency and enabling features like event publishing for all log sources.

---

## ✅ Action Plan

### 1. Dependency Management

The Anki add-on needs the ability to make HTTP requests. We will use the standard `requests` library.

- **1. Package the `requests` library with the add-on.**
  - Anki add-ons bundle their dependencies. In the `AnkiLogger` directory, install the library directly into the folder.
  - Run the command: `pip install -t . requests`
  - Ensure the new `requests` and `urllib3` (etc.) directories are included when packaging the add-on.

### 2. Configuration

To avoid hardcoding the API endpoint, we will make it configurable.

- **1. Add `api_base_url` to the configuration.**
  - In `config.json`, add a new top-level key for the API URL.

  ```json
  {
    "api_base_url": "http://localhost:3000",
    "time_frame": {
      "Japanese": 4
    }
  }
  ```

- **2. Update `config.py` to load and expose the new setting.**
  - Modify the `Configuration` class to load `api_base_url`.
  - Provide a method to access it, e.g., `get_api_base_url() -> str`.
  - Ensure it handles the case where the key might be missing in older configs by providing a sensible default (`"http://localhost:3000"`).

### 3. Create the API Client (The Adapter)

We will create a dedicated module to encapsulate all knowledge of the web API. This follows the **Adapter** pattern, separating the "how" of communication from the "what" of our application logic.

- **1. Create a new file: `api_client.py`.**
  - This module's sole responsibility is to handle HTTP communication with the `lifestats-web-api`.

- **2. Implement an asynchronous logging function.**
  - Define a function `log_statistic_value_async(statistic_id: int, date: date, value: str, update_policy: str | None)`.
  - This function should:
    - Import the global `application_config` and get the `api_base_url`.
    - Construct the full URL: `{base_url}/statistics/{statistic_id}/log/{date_str}`.
    - Build the JSON payload. Note that `update_policy` should be omitted from the JSON if it is `None`.
    ```python
    payload = {"value": value}
    if update_policy:
        payload["update_policy"] = update_policy
    ```

    - Use `ThreadPoolExecutor` (already present in `statistic_logging.py`) to execute a `requests.post()` call in a separate thread, keeping the Anki UI responsive.
    - Add logging for both success and error cases (e.g., connection errors, non-2xx status codes).

### 4. Refactor Logging Logic to Use the API Client

Now, we'll replace the old `subprocess` calls with our new, clean API client.

- **1. Update `statistic_logging.py` to use the new `api_client`.**
  - Import the `log_statistic_value_async` function from `api_client.py`.
  - **Completely remove** the `execute_shell_command_async` function. Its responsibility is now fully handled by the `api_client`.

- **2. Rewrite `log_time_until_now`.**
  - Its signature remains the same, but the implementation will change.
  - Instead of building a shell command, it will now call `api_client.log_statistic_value_async`.
  - **Value formatting:** The value must be formatted as a string `HH:MM:SS-HH:MM:SS`.
  - **Update Policy:** The `update_policy` will be the string `"updateendvalue"`.

  ```python
  # statistic_logging.py

  from datetime import datetime
  from . import api_client # NEW import

  def log_time_until_now(statistic_id: int, start: datetime):
      end = datetime.now()
      date = end.date() # Log for the current date
      value_str = f'{start.strftime("%H:%M:%S")}-{end.strftime("%H:%M:%S")}'

      api_client.log_statistic_value_async(
          statistic_id=statistic_id,
          date=date,
          value=value_str,
          update_policy="updateendvalue"
      )
  ```

- **3. Rewrite `log_current_time`.**
  - This function will also be updated to call the new `api_client`. The `update_policy` for this call should be `None`.

### 5. Verification & Cleanup

- **1. Verify the entire flow.**
  - Start the `lifestats-web-api` server locally.
  - Start Anki with the modified add-on.
  - Perform a study session that is configured to be logged.
  - **Observe:** Watch the `lifestats-web-api` console logs. You should see incoming `POST` requests to `/statistics/{id}/log/{date}`.
  - **Confirm:** Check the SQLite database to ensure the time spans were correctly logged and updated.

- **2. Update user documentation (if any).**
  - The add-on no longer requires the `log_statistic` binary to be in the system's `PATH`. This simplifies setup and should be communicated.
