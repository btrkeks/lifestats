"""Configuration manager for AnkiLogger plugin.

This module manages the lifecycle of the addon configuration file.
It is a pure persistence adapter with no business logic.
"""

from typing import Dict, Optional
from dataclasses import asdict

from aqt import mw
from anki.decks import DeckId

from .logger_config import get_logger
from .api_client import StatisticBundle

logger = get_logger(__name__)

ADDON_PATH = __file__
ADDON_NAME = "AnkiLogger"


class ConfigManager:
    """Manages addon configuration as a pure persistence adapter.

    This class handles only reading and writing the config.json file.
    It has no knowledge of the API or provisioning logic.
    """

    def __init__(self):
        self._config = self._load_config()

    def _load_config(self) -> Dict:
        """Loads config from Anki, providing defaults if none exists."""
        config = mw.addonManager.getConfig(ADDON_NAME)
        if config is None:
            logger.warning("No configuration found, creating default config")
            config = self._get_default_config()
            self.save()
        else:
            logger.debug(f"Configuration loaded from disk")
            # Perform migration if needed
            config = self._migrate_config_if_needed(config)

        logger.info(f"Configuration loaded: {len(config.get('deck_configs', {}))} deck bundles")
        return config

    def _get_default_config(self) -> Dict:
        """Returns default configuration structure."""
        return {
            "api_base_url": "http://localhost:3000",
            "deck_configs": {}
        }

    def _migrate_config_if_needed(self, config: Dict) -> Dict:
        """Migrates old name-based config to did-based config if needed."""
        # Check if old format exists (time_frame key)
        if "time_frame" in config and config["time_frame"]:
            logger.info("Detected old config format, will migrate on first use")
            # Keep both formats during transition
            if "deck_configs" not in config:
                config["deck_configs"] = {}

        # Ensure deck_configs exists
        if "deck_configs" not in config:
            config["deck_configs"] = {}

        return config

    def save(self):
        """Saves the current configuration state to disk."""
        mw.addonManager.writeConfig(ADDON_NAME, self._config)
        logger.debug("Configuration saved")

    def get_api_base_url(self) -> str:
        """Returns the configured API base URL."""
        return self._config.get("api_base_url", "http://localhost:3000")

    def get_deck_bundle(self, deck_id: DeckId) -> Optional[StatisticBundle]:
        """Retrieves the statistic bundle for a deck from the cache.

        Args:
            deck_id: The Anki deck ID (did)

        Returns:
            StatisticBundle if found, None otherwise
        """
        if deck_id is None:
            return None

        deck_configs = self._config.get("deck_configs", {})
        deck_conf = deck_configs.get(str(deck_id))

        if deck_conf:
            logger.debug(f"Found cached bundle for deck_id={deck_id}")
            return StatisticBundle(**deck_conf)

        return None

    def save_deck_bundle(self, deck_id: DeckId, bundle: StatisticBundle):
        """Saves a statistic bundle for a deck to the cache and persists it.

        Args:
            deck_id: The Anki deck ID (did)
            bundle: The StatisticBundle to save
        """
        if "deck_configs" not in self._config:
            self._config["deck_configs"] = {}

        self._config["deck_configs"][str(deck_id)] = asdict(bundle)
        self.save()
        logger.info(f"Saved bundle for deck_id={deck_id}: {bundle}")

    def get_all_bundles(self) -> Dict[str, Dict]:
        """Returns all deck bundles from the config.

        Returns:
            Dictionary mapping deck_id (as string) to bundle dict
        """
        return dict(self._config.get("deck_configs", {}))
