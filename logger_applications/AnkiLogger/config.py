from typing import Dict, Optional
import os

from aqt import mw
from anki.decks import DeckId

from .anki_utils import get_deck_name

DeckStatisticMapping = Dict[str, int]

ADDON_PATH = os.path.dirname(os.path.abspath(__file__))
ADDON_NAME = os.path.basename(ADDON_PATH)


class Configuration:
    """Manages the addon configuration for deck to statistic mappings.

    The configuration is stored in JSON format with the following structure:
    {
        "time_frame": {
            "Japanese": 4,
            "English::Vocabulary": 5,
            "Math": 8
        }
    }
    """

    def __init__(self) -> None:
        self._config: DeckStatisticMapping = {}
        self._load_config()

    def _load_config(self) -> None:
        """Load configuration from disk. Creates default if none exists."""
        config = mw.addonManager.getConfig(ADDON_NAME)
        if config is None:
            # Create default config if none exists
            config = {"time_frame": {}}
            self._save_config(config)

        self._config = config.get("time_frame", {})

    @staticmethod
    def _save_config(config: dict) -> None:
        """Save configuration to disk."""
        mw.addonManager.writeConfig(ADDON_NAME, config)

    def get_statistic_id_for_deck(self, deck_id: DeckId) -> Optional[int]:
        """Get the configured statistic ID for a given deck ID.

        Args:
            deck_id: The Anki deck ID to look up

        Returns:
            The configured statistic ID or None if no mapping exists
        """
        if deck_id is None:
            return None

        deck_name = get_deck_name(deck_id)
        return self._config.get(deck_name)

    def set_deck_statistic_mapping(self, deck_name: str, statistic_id: int) -> None:
        """Configure a mapping between a deck name and statistic ID.

        Args:
            deck_name: The name of the deck to map
            statistic_id: The statistic ID to map to
        """
        self._config[deck_name] = statistic_id
        self._save_config({"time_frame": self._config})

    def remove_deck_statistic_mapping(self, deck_name: str) -> None:
        """Remove a deck mapping from the configuration.

        Args:
            deck_name: The name of the deck to remove mapping for
        """
        if deck_name in self._config:
            del self._config[deck_name]
            self._save_config({"time_frame": self._config})

    def get_all_mappings(self) -> DeckStatisticMapping:
        """Get all configured deck to statistic mappings.

        Returns:
            A dictionary of deck names to statistic IDs
        """
        return dict(self._config)


# Global configuration instance
application_config = Configuration()


# Convenience function for getting statistic ID
def get_statistic_id_for_deck(deck_id: DeckId) -> Optional[int]:
    """Convenience function to get statistic ID for a deck.

    Args:
        deck_id: The Anki deck ID to look up

    Returns:
        The configured statistic ID or None if no mapping exists
    """
    return application_config.get_statistic_id_for_deck(deck_id)
