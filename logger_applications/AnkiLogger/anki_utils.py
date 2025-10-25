from anki.decks import DeckId
from aqt import mw


def deck_is_finished(deck_id: DeckId) -> bool:
    if deck_id is None:
        return False

    counts = list(mw.col.sched.counts())  # (new, learning, review)

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
        return mw.col.decks.get(deck_id)["name"]


# def get_total_backlog_count(deck_id: DeckId) -> int:
#     """Get the total number of cards due (Learning + Review).
#
#     Args:
#         deck_id: The Anki deck ID
#
#     Returns:
#         Sum of cards in new, learning, and review queues.
#     """
#     if deck_id is None:
#         return 0
#
#     # Select the deck temporarily to get its counts
#     original_deck = mw.col.decks.selected()
#     try:
#         mw.col.decks.select(deck_id)
#         # counts() returns a tuple: (new, learning, review) due today
#         counts = mw.col.sched.counts()
#
#         # Sum them up to get the total backlog
#         total_backlog = counts[1] + counts[2]
#
#         return total_backlog
#     finally:
#         # Restore original deck selection
#         mw.col.decks.select(original_deck)


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
