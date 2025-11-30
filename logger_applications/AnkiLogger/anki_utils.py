from anki.decks import DeckId
from aqt import mw

from .logger_config import get_logger

logger = get_logger(__name__)


def deck_is_finished(deck_id: DeckId) -> bool:
    """Check if a specific deck has no more cards due today."""
    if deck_id is None:
        return False

    counts = list(mw.col.sched.counts())  # (new, learning, review)

    if all(count == 0 for count in counts):
        return True
    else:
        return False


def all_cards_finished() -> bool:
    """Check if ALL cards across ALL decks are finished for today.

    This is the global "congratulations" state - when Anki would show
    the congratulations screen because there are no more cards to review
    across the entire collection.

    Returns:
        True if all cards are done, False otherwise
    """
    counts = list(mw.col.sched.counts())  # (new, learning, review) for current deck

    # If the scheduler returns all zeros, it means no cards are due
    # This works because Anki's scheduler considers all decks when in overview
    if all(count == 0 for count in counts):
        return True

    return False


def get_current_deck_id() -> DeckId:
    return mw.col.decks.selected()


def get_deck_name(deck_id: DeckId) -> str:
    if deck_id is None:
        return ""
    else:
        return mw.col.decks.get(deck_id)["name"]


def get_total_backlog_count(deck_id: DeckId) -> int:
    """Get the UNCAPPED total number of cards due (Learning + Review).

    Args:
        deck_id: The Anki deck ID

    Returns:
        Sum of uncapped cards in learning and review queues (no daily limits applied).
    """
    if deck_id is None:
        return 0

    # Get the deck tree with counts (includes both capped and uncapped)
    tree = mw.col.sched.deck_due_tree()

    # Find the specific deck in the tree
    deck_node = mw.col.decks.find_deck_in_tree(tree, deck_id)

    if deck_node is None:
        return 0

    # Sum uncapped learning + review cards
    # - review_uncapped: review cards in this deck (no limits)
    # - interday_learning_uncapped: cards due between today and tomorrow
    # - intraday_learning: cards learned today, still in learning phase
    total_backlog = (
        deck_node.review_uncapped
        + deck_node.interday_learning_uncapped
        + deck_node.intraday_learning
    )

    return total_backlog


def get_cards_studied_today(deck_id: DeckId | None = None) -> int:
    """Get the number of unique cards reviewed today, optionally filtered by deck."""
    today_start_ms = (mw.col.sched.day_cutoff - 86400) * 1000

    if deck_id is None:
        # All decks
        count = mw.col.db.scalar(
            """
            SELECT COUNT(DISTINCT cid)
            FROM revlog
            WHERE id > ?
              AND type NOT IN (4, 5)
            """,
            today_start_ms,
        )
    else:
        # Specific deck (including subdecks)
        deck_ids = [deck_id] + [d.id for d in mw.col.decks.children(deck_id)]
        placeholders = ",".join("?" * len(deck_ids))

        count = mw.col.db.scalar(
            f"""
            SELECT COUNT(DISTINCT r.cid)
            FROM revlog r
            JOIN cards c ON r.cid = c.id
            WHERE r.id > ?
              AND r.type NOT IN (4, 5)
              AND c.did IN ({placeholders})
            """,
            today_start_ms,
            *deck_ids,
        )

    return count or 0
