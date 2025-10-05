from typing import Dict, Optional
import os

from aqt import mw
from anki.decks import DeckId

from .logger_config import get_logger
from .anki_utils import get_deck_name

logger = get_logger(__name__)

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
        logger.info("Initializing Configuration")
        self._config: DeckStatisticMapping = {}
        self._api_base_url: str = "http://localhost:3000"
        self._load_config()
        logger.info(f"Configuration loaded: {len(self._config)} deck mappings, API URL: {self._api_base_url}")

    def _load_config(self) -> None:
        """Load configuration from disk. Creates default if none exists."""
        logger.debug(f"Loading configuration for addon: {ADDON_NAME}")
        config = mw.addonManager.getConfig(ADDON_NAME)

        if config is None:
            logger.warning("No configuration found, creating default config")
            # Create default config if none exists
            config = {"api_base_url": "http://localhost:3000", "time_frame": {}}
            self._save_config(config)
        else:
            logger.debug(f"Configuration loaded from disk: {config}")

        self._config = config.get("time_frame", {})
        self._api_base_url = config.get("api_base_url", "http://localhost:3000")
        logger.info(f"=== CONFIGURED DECK MAPPINGS ===")
        if self._config:
            for deck_name, stat_id in self._config.items():
                logger.info(f"  '{deck_name}' -> statistic_id {stat_id}")
        else:
            logger.warning("No deck mappings configured!")
        logger.info(f"API base URL: {self._api_base_url}")

    @staticmethod
    def _save_config(config: dict) -> None:
        """Save configuration to disk."""
        logger.debug(f"Saving configuration: {config}")
        mw.addonManager.writeConfig(ADDON_NAME, config)
        logger.debug("Configuration saved successfully")

    def get_statistic_id_for_deck(self, deck_id: DeckId) -> Optional[int]:
        """Get the configured statistic ID for a given deck ID.

        Args:
            deck_id: The Anki deck ID to look up

        Returns:
            The configured statistic ID or None if no mapping exists
        """
        if deck_id is None:
            logger.debug("get_statistic_id_for_deck called with None deck_id")
            return None

        deck_name = get_deck_name(deck_id)
        statistic_id = self._config.get(deck_name)
        logger.debug(f"get_statistic_id_for_deck: deck_id={deck_id}, deck_name={deck_name}, statistic_id={statistic_id}")
        return statistic_id

    def set_deck_statistic_mapping(self, deck_name: str, statistic_id: int) -> None:
        """Configure a mapping between a deck name and statistic ID.

        Args:
            deck_name: The name of the deck to map
            statistic_id: The statistic ID to map to
        """
        logger.info(f"Setting deck mapping: deck_name={deck_name}, statistic_id={statistic_id}")
        self._config[deck_name] = statistic_id
        self._save_config({"time_frame": self._config})

    def remove_deck_statistic_mapping(self, deck_name: str) -> None:
        """Remove a deck mapping from the configuration.

        Args:
            deck_name: The name of the deck to remove mapping for
        """
        logger.info(f"Removing deck mapping: deck_name={deck_name}")
        if deck_name in self._config:
            del self._config[deck_name]
            self._save_config({"time_frame": self._config})
        else:
            logger.warning(f"Attempted to remove non-existent mapping for deck: {deck_name}")

    def get_all_mappings(self) -> DeckStatisticMapping:
        """Get all configured deck to statistic mappings.

        Returns:
            A dictionary of deck names to statistic IDs
        """
        return dict(self._config)

    def get_api_base_url(self) -> str:
        """Get the configured API base URL.

        Returns:
            The configured API base URL (defaults to http://localhost:3000)
        """
        return self._api_base_url


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
