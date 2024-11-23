import subprocess
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime
from typing import List
from anki.decks import DeckId
from aqt.utils import tooltip

executor = ThreadPoolExecutor(max_workers=5)


def execute_shell_command_async(cmd: List[str]):
    def run_command():
        try:
            subprocess.run(cmd, check=True)
        except subprocess.CalledProcessError as e:
            tooltip(f"Error calling {cmd}: {e}")

    executor.submit(run_command)


def log_current_time(statistic_id: int):
    execute_shell_command_async(["log_statistic", f"--id={statistic_id}", "--now"])


def log_time_until_now(statistic_id: int, start: datetime):
    end = datetime.now()
    execute_shell_command_async(
        ["log_statistic",
         f"--id={statistic_id}",
         f'--value={start.strftime("%H:%M:%S")}-{end.strftime("%H:%M:%S")}',
         "--update-policy=overwritelast"
        ]
    )
