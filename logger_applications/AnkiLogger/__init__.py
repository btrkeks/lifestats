"""AnkiLogger addon entry point.

This module initializes the addon and registers hooks after Anki is fully loaded.
"""

import aqt

from .logger_config import get_logger
from .addon_manager import AddonManager
from .hooks import register_hooks

logger = get_logger(__name__)


def init():
    """Initialize the addon after Anki is fully loaded.

    Creates the AddonManager which wires up all dependencies,
    then registers Anki hooks with the session tracker.
    """
    logger.info("=== Initializing AnkiLogger addon (refactored architecture) ===")
    try:
        # Create the addon manager (composition root)
        addon_manager = AddonManager()
        logger.debug("AddonManager created")

        # Get the session tracker
        session_tracker = addon_manager.get_session_tracker()
        logger.debug("SessionTracker retrieved")

        # Register hooks with the session tracker
        register_hooks(session_tracker)

        logger.info("=== AnkiLogger addon initialized successfully ===")
    except Exception as e:
        logger.error(f"Error initializing addon: {e}", exc_info=True)
        raise


# Register the initialization function to run after Anki's main window is ready
aqt.gui_hooks.main_window_did_init.append(init)
