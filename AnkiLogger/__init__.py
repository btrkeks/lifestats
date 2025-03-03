from typing import Literal
from anki.cards import Card
import aqt
from aqt import gui_hooks, reviewer
from anki.decks import DeckId

from .anki_study_time_logger import AnkiLogger
from .anki_utils import deck_is_finished, get_deck_name, get_current_deck_id
from .statistic_logging import log_current_time

def log_finished_deck(deck_id: DeckId):
    """Logs the current time for a finished deck if a statistic ID is configured."""
    if get_deck_name(deck_id) == "Japanese":
        log_current_time(26)

def on_card_answered(anki_logger: AnkiLogger, reviewer: reviewer.Reviewer, card: Card, ease: Literal[1, 2, 3, 4]):
    """Event handler for card answer."""
    anki_logger.ping()

def on_show_question(anki_logger: AnkiLogger, card: Card):
    """Event handler for showing a question."""
    anki_logger.ping()

def on_show_answer(anki_logger: AnkiLogger, card: Card):
    """Event handler for showing an answer."""
    anki_logger.ping()

def on_state_change(anki_logger: AnkiLogger, new_state: aqt.main.MainWindowState, old_state: aqt.main.MainWindowState):
    """Event handler for state changes, triggering a ping on review start."""
    if new_state == "review":
        anki_logger.ping()

def on_reviewer_will_end(anki_logger: AnkiLogger):
    anki_logger.stop_running()

    current_deck_id = get_current_deck_id()
    if deck_is_finished(current_deck_id):
        log_finished_deck(current_deck_id)

def register_hooks(anki_logger: AnkiLogger):
    """Registers the Anki hooks with the provided logger instance."""
    gui_hooks.reviewer_did_answer_card.append(lambda r, c, e: on_card_answered(anki_logger, r, c, e))
    gui_hooks.reviewer_did_show_question.append(lambda c: on_show_question(anki_logger, c))
    gui_hooks.reviewer_did_show_answer.append(lambda c: on_show_answer(anki_logger, c))
    gui_hooks.state_did_change.append(lambda s, o: on_state_change(anki_logger, s, o))
    gui_hooks.reviewer_will_end.append(lambda: on_reviewer_will_end(anki_logger))

def init():
    """Initialize the addon after Anki is fully loaded."""
    anki_logger = AnkiLogger()
    register_hooks(anki_logger)

aqt.gui_hooks.main_window_did_init.append(init)