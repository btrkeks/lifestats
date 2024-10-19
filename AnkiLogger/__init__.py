import subprocess
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime
from typing import Literal, List
from threading import Timer

from anki.cards import Card
import aqt
from aqt import gui_hooks, reviewer, mw
from aqt.utils import tooltip
from anki.decks import DeckId

from . import util

# config = mw.addonManager.getConfig(__name__)
# deck_name = config["deck_name"]

executor = ThreadPoolExecutor(max_workers=5)


def run_command_async(cmd: List[str]):
    def run_command():
        try:
            subprocess.run(cmd, check=True)
        except subprocess.CalledProcessError as e:
            tooltip(f"Error calling {cmd}: {e}")

    executor.submit(run_command)


def log_deck_finished():
    tooltip("Logging finished deck")
    run_command_async(["log_statistic", "--id=26", "--now"])

def log_time_frame(start: datetime, end: datetime):
    tooltip("Logging timeframe")
    run_command_async(["log_statistic", "--id=4", f'--value={start.strftime("%H:%M:%S")}-{end.strftime("%H:%M:%S")}'])


class AnkiLogger:
    def __init__(self):
        self.is_running: bool = False
        self.start_time = None
        self.timer = None
        self.current_deck_id = None

    def _time_expired(self):
        self.stop_running()

    def get_current_deck_id(self):
        return self.current_deck_id

    def add_finished_deck(self, deck_id: DeckId):
        if util.get_deck_name_from_id(deck_id) == "Japanese":
            log_deck_finished()

    def start_running(self):
        current_deck_id = util.get_current_deck_id()

        tooltip("Start logging", period=3000)
        self._start_timer()

        self.is_running = True
        self.start_time = datetime.now()
        self.current_deck_id = current_deck_id

    def stop_running(self):
        if not self.is_running:
            return

        mw.taskman.run_on_main(lambda: tooltip("Stop logging", period=3000))

        self._stop_timer()
        log_time_frame(self.start_time, datetime.now())

        self.is_running = False
        self.start_time = None
        self.current_deck_id = None

    def _start_timer(self):
        self.timer = Timer(60, self._time_expired)
        self.timer.start()

    def _stop_timer(self):
        if self.timer:
            self.timer.cancel()

    def _reset_timer(self):
        self._stop_timer()
        self._start_timer()

    def ping(self):
        if not self.is_running:
            self.start_running()
        else:
            self._reset_timer()


anki_logger = AnkiLogger()


def on_card_answered(reviewer: reviewer.Reviewer, card: Card, ease: Literal[1, 2, 3, 4]):
    anki_logger.ping()


def on_show_question(card: Card):
    anki_logger.ping()


def on_show_answer(card: Card):
    anki_logger.ping()


def on_state_change(new_state: aqt.main.MainWindowState, old_state: aqt.main.MainWindowState):
    if new_state == "review":
        anki_logger.ping()


def on_reviewer_will_end():
    current_deck_id = anki_logger.get_current_deck_id()
    anki_logger.stop_running()
    finished = util.check_deck_is_finished(current_deck_id)
    if finished:
        anki_logger.add_finished_deck(current_deck_id)


gui_hooks.reviewer_did_answer_card.append(on_card_answered)
gui_hooks.reviewer_did_show_question.append(on_show_question)
gui_hooks.reviewer_did_show_answer.append(on_show_answer)

gui_hooks.state_did_change.append(on_state_change)
gui_hooks.reviewer_will_end.append(on_reviewer_will_end)
