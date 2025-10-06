from aqt import mw
from anki.decks import DeckId


def deck_is_finished(deck_id: DeckId) -> bool:
    if deck_id is None:
        return False

    counts = list(mw.col.sched.counts()) # (new, learning, review)

    if all(count == 0 for count in counts):
        return True
    else:
        return False

def get_current_deck_id() -> DeckId:
    return mw.col.decks.selected()

def get_deck_name(deck_id: DeckId) -> str:
    if deck_id is None:
        return ""
    else:
        return mw.col.decks.get(deck_id)['name']

def get_total_backlog_count(deck_id: DeckId) -> int:
    """Get the total number of cards due (New + Learning + Review).

    Args:
        deck_id: The Anki deck ID

    Returns:
        Sum of cards in new, learning, and review queues.
    """
    if deck_id is None:
        return 0

    # Select the deck temporarily to get its counts
    original_deck = mw.col.decks.selected()
    try:
        mw.col.decks.select(deck_id)
        # counts() returns a tuple: (new, learning, review) due today
        counts = mw.col.sched.counts()

        # Sum them up to get the total backlog
        total_backlog = counts[0] + counts[1] + counts[2]

        return total_backlog
    finally:
        # Restore original deck selection
        mw.col.decks.select(original_deck)