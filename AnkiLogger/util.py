from anki.decks import DeckId
from aqt import mw

def check_deck_is_finished(deck_id) -> bool:
    if deck_id is None:
        return False

    counts = list(mw.col.sched.counts()) # (new, learning, review)

    if all(count == 0 for count in counts):
        return True
    else:
        return False

def get_current_deck_id():
    return mw.col.decks.selected()

def get_deck_name_from_id(deck_id) -> str:
    if deck_id is None:
        return ""
    else:
        return mw.col.decks.get(deck_id)['name']