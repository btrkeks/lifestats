from typing import Literal
from anki.cards import Card
import aqt
from aqt import gui_hooks, reviewer
from anki.decks import DeckId

from .logger_config import get_logger
from .anki_study_time_logger import AnkiLogger
from .anki_utils import deck_is_finished, get_deck_name, get_current_deck_id
from .statistic_logging import log_current_time

logger = get_logger(__name__)

def log_finished_deck(deck_id: DeckId):
    """Logs the current time for a finished deck if a statistic ID is configured."""
    logger.info(f"log_finished_deck called for deck_id={deck_id}")
    deck_name = get_deck_name(deck_id)
    logger.debug(f"Deck name: {deck_name}")

    if deck_name == "日本語":
        logger.info(f"Deck '{deck_name}' finished, logging current time to statistic 26")
        log_current_time(26)
    else:
        logger.debug(f"Deck '{deck_name}' does not match '日本語', skipping log")

def on_card_answered(anki_logger: AnkiLogger, reviewer: reviewer.Reviewer, card: Card, ease: Literal[1, 2, 3, 4]):
    """Event handler for card answer."""
    logger.debug(f"on_card_answered: card_id={card.id}, ease={ease}")
    anki_logger.ping()

def on_show_question(anki_logger: AnkiLogger, card: Card):
    """Event handler for showing a question."""
    logger.debug(f"on_show_question: card_id={card.id}")
    anki_logger.ping()

def on_show_answer(anki_logger: AnkiLogger, card: Card):
    """Event handler for showing an answer."""
    logger.debug(f"on_show_answer: card_id={card.id}")
    anki_logger.ping()

def on_state_change(anki_logger: AnkiLogger, new_state: aqt.main.MainWindowState, old_state: aqt.main.MainWindowState):
    """Event handler for state changes, triggering a ping on review start."""
    logger.info(f"on_state_change: old_state={old_state}, new_state={new_state}")
    if new_state == "review":
        logger.info("Entering review state, triggering ping")
        anki_logger.ping()

def on_reviewer_will_end(anki_logger: AnkiLogger):
    logger.info("on_reviewer_will_end: reviewer ending")
    anki_logger.stop_running()

    current_deck_id = get_current_deck_id()
    logger.debug(f"Current deck_id: {current_deck_id}")
    if deck_is_finished(current_deck_id):
        logger.info(f"Deck {current_deck_id} is finished")
        log_finished_deck(current_deck_id)
    else:
        logger.debug(f"Deck {current_deck_id} is not finished")

def register_hooks(anki_logger: AnkiLogger):
    """Registers the Anki hooks with the provided logger instance."""
    logger.info("Registering Anki hooks")
    try:
        gui_hooks.reviewer_did_answer_card.append(lambda r, c, e: on_card_answered(anki_logger, r, c, e))
        logger.debug("Registered reviewer_did_answer_card hook")

        gui_hooks.reviewer_did_show_question.append(lambda c: on_show_question(anki_logger, c))
        logger.debug("Registered reviewer_did_show_question hook")

        gui_hooks.reviewer_did_show_answer.append(lambda c: on_show_answer(anki_logger, c))
        logger.debug("Registered reviewer_did_show_answer hook")

        gui_hooks.state_did_change.append(lambda s, o: on_state_change(anki_logger, s, o))
        logger.debug("Registered state_did_change hook")

        gui_hooks.reviewer_will_end.append(lambda: on_reviewer_will_end(anki_logger))
        logger.debug("Registered reviewer_will_end hook")

        logger.info("All hooks registered successfully")
    except Exception as e:
        logger.error(f"Error registering hooks: {e}", exc_info=True)
        raise

def init():
    """Initialize the addon after Anki is fully loaded."""
    logger.info("Initializing AnkiLogger addon")
    try:
        anki_logger = AnkiLogger()
        logger.debug("AnkiLogger instance created")

        register_hooks(anki_logger)
        logger.info("AnkiLogger addon initialized successfully")
    except Exception as e:
        logger.error(f"Error initializing addon: {e}", exc_info=True)
        raise

aqt.gui_hooks.main_window_did_init.append(init)
