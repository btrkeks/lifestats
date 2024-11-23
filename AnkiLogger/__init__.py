from typing import Literal
from anki.cards import Card
import aqt
from aqt import gui_hooks, reviewer
from anki.decks import DeckId

from .anki_study_time_logger import AnkiLogger
from .anki_utils import get_deck_finished, get_deck_name, get_current_deck_id
from .statistic_logging import log_current_time

anki_logger: AnkiLogger | None = None

def log_finished_deck_with_id(deck_id: DeckId):
    if get_deck_name(deck_id) == "Japanese":
        log_current_time(26)


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
    anki_logger.stop_running()

    current_deck_id = get_current_deck_id()
    if get_deck_finished(current_deck_id):
        log_finished_deck_with_id(current_deck_id)


def register_hooks():
    gui_hooks.reviewer_did_answer_card.append(on_card_answered)
    gui_hooks.reviewer_did_show_question.append(on_show_question)
    gui_hooks.reviewer_did_show_answer.append(on_show_answer)
    gui_hooks.state_did_change.append(on_state_change)
    gui_hooks.reviewer_will_end.append(on_reviewer_will_end)

def init() -> None:
    """Initialize the addon after Anki is fully loaded."""
    global anki_logger
    anki_logger = AnkiLogger()
    register_hooks()

aqt.gui_hooks.main_window_did_init.append(init)