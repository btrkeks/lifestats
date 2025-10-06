"""Addon manager for wiring up dependencies.

This module is the composition root that instantiates all components
and wires them together with dependency injection.
"""

from .logger_config import get_logger
from .api_client import ApiClient
from .config_manager import ConfigManager
from .session_tracker import SessionTracker

logger = get_logger(__name__)


class AddonManager:
    """Main orchestrator for the AnkiLogger addon.

    Instantiates and wires together all components following
    the dependency injection pattern.
    """

    def __init__(self):
        logger.info("Initializing AddonManager")

        # 1. Create ConfigManager first. It has no dependencies.
        self.config_manager = ConfigManager()
        logger.debug("ConfigManager created")

        # 2. Get the API URL from the single source of truth.
        api_base_url = self.config_manager.get_api_base_url()
        logger.debug(f"API base URL from config: {api_base_url}")

        # 3. Create ApiClient with the URL.
        self.api_client = ApiClient(base_url=api_base_url, timeout=10)
        logger.debug("ApiClient created")

        # 4. Create SessionTracker and inject dependencies.
        self.session_tracker = SessionTracker(
            api_client=self.api_client,
            config_manager=self.config_manager,
            idle_timeout_seconds=60,
            flush_interval_seconds=120
        )
        logger.debug("SessionTracker created")

        logger.info("AddonManager initialization complete")

    def get_session_tracker(self) -> SessionTracker:
        """Returns the session tracker instance."""
        return self.session_tracker

    def get_api_client(self) -> ApiClient:
        """Returns the API client instance."""
        return self.api_client

    def get_config_manager(self) -> ConfigManager:
        """Returns the config manager instance."""
        return self.config_manager
