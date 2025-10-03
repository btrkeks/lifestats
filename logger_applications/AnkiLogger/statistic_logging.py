import subprocess
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime
from typing import List
from anki.decks import DeckId
from aqt.utils import tooltip, showWarning
import sys

executor = ThreadPoolExecutor(max_workers=5)


def execute_shell_command_async(cmd: List[str]):
    def run_command():
        try:
            print(f"Executing command: {' '.join(cmd)}")
            result = subprocess.run(
                cmd,
                check=True,
                text=True,  # Ensures output is in string format
                capture_output=True  # Captures stdout and stderr
            )
            if result.stdout:
                print(f"Command output: {result.stdout}")
            print("Command executed successfully")
        except subprocess.CalledProcessError as e:
            error_message = (
                f"Error calling {cmd}: {e}\n"
                f"Return Code: {e.returncode}\n"
                f"Output: {e.stdout}\n"
                f"Error: {e.stderr}"
            )
            print(error_message, file=sys.stderr)
        except Exception as e:
            print(f"Unexpected error: {str(e)}", file=sys.stderr)

    executor.submit(run_command)


def log_current_time(statistic_id: int):
    print(f"Attempting to log current time for statistic_id: {statistic_id}")
    execute_shell_command_async(["log_statistic", f"--id={statistic_id}", "--now"])


def log_time_until_now(statistic_id: int, start: datetime):
    end = datetime.now()
    execute_shell_command_async(
        ["log_statistic",
         f"--id={statistic_id}",
         f'--value={start.strftime("%H:%M:%S")}-{end.strftime("%H:%M:%S")}',
         "--update-policy=updateendvalue"
        ]
    )
