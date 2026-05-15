"""Centralized hook registration for Anki events.

This module contains all the hook registration logic and event handlers.
"""

from datetime import datetime
from typing import Literal

import aqt
from anki.cards import Card
from anki.decks import DeckId
from aqt import gui_hooks, reviewer

from .anki_utils import (
    deck_is_finished,
    get_cards_studied_today,
    get_current_deck_id,
    get_deck_name,
)
from .logger_config import get_logger
from .session_tracker import SessionTracker

logger = get_logger(__name__)


def on_card_answered(
    session_tracker: SessionTracker,
    reviewer: reviewer.Reviewer,
    card: Card,
    ease: Literal[1, 2, 3, 4],
):
    """Event handler for card answer."""
    logger.debug(f"on_card_answered: card_id={card.id}, ease={ease}")
    session_tracker.ping()


def on_show_question(session_tracker: SessionTracker, card: Card):
    """Event handler for showing a question."""
    logger.debug(f"on_show_question: card_id={card.id}")
    session_tracker.ping()


def on_show_answer(session_tracker: SessionTracker, card: Card):
    """Event handler for showing an answer."""
    logger.debug(f"on_show_answer: card_id={card.id}")
    session_tracker.ping()


def on_state_change(
    session_tracker: SessionTracker,
    new_state: aqt.main.MainWindowState,
    old_state: aqt.main.MainWindowState,
):
    """Event handler for state changes, triggering a ping on review start."""
    logger.info(f"on_state_change: old_state={old_state}, new_state={new_state}")
    if new_state == "deckBrowser":
        session_tracker.log_startup_backlogs()

    if new_state == "review":
        logger.info("Entering review state, triggering ping")
        session_tracker.ping()


def on_reviewer_will_end(session_tracker: SessionTracker):
    """Event handler for reviewer ending."""
    logger.info("on_reviewer_will_end: reviewer ending")

    current_deck_id = get_current_deck_id()
    logger.debug(f"Current deck_id: {current_deck_id}")

    # TEST: Log cards studied today
    cards_studied = get_cards_studied_today(DeckId(1661059140274))
    logger.info(f"TEST: Cards studied today = {cards_studied}")

    # Check if current deck is finished BEFORE stopping (so bundle still exists)
    if deck_is_finished(current_deck_id):
        logger.info(f"Deck {current_deck_id} is finished - logging finished time")
        session_tracker.log_finished_time()
    else:
        logger.debug(f"Deck {current_deck_id} is not finished")

    # Now stop the session (this clears the bundle)
    session_tracker.stop_running()


def register_hooks(session_tracker: SessionTracker):
    """Registers the Anki hooks with the provided session tracker instance.

    Args:
        session_tracker: The SessionTracker instance to use for event handling
    """
    logger.info("Registering Anki hooks")
    try:
        gui_hooks.reviewer_did_answer_card.append(
            lambda r, c, e: on_card_answered(session_tracker, r, c, e)
        )
        logger.debug("Registered reviewer_did_answer_card hook")

        gui_hooks.reviewer_did_show_question.append(
            lambda c: on_show_question(session_tracker, c)
        )
        logger.debug("Registered reviewer_did_show_question hook")

        gui_hooks.reviewer_did_show_answer.append(
            lambda c: on_show_answer(session_tracker, c)
        )
        logger.debug("Registered reviewer_did_show_answer hook")

        gui_hooks.state_did_change.append(
            lambda s, o: on_state_change(session_tracker, s, o)
        )
        logger.debug("Registered state_did_change hook")

        gui_hooks.reviewer_will_end.append(
            lambda: on_reviewer_will_end(session_tracker)
        )
        logger.debug("Registered reviewer_will_end hook")

        logger.info("All hooks registered successfully")
    except Exception as e:
        logger.error(f"Error registering hooks: {e}", exc_info=True)
        raise
