"""Logging configuration for the AnkiLogger addon.

Provides centralized logging setup with file-based logging to help debug issues.
"""

import logging
import os
from logging.handlers import RotatingFileHandler


# Determine log file location
ADDON_PATH = os.path.dirname(os.path.abspath(__file__))
LOG_FILE = os.path.join(ADDON_PATH, 'anki_logger.log')

# Configure root logger for this addon
_configured = False


def setup_logging(level=logging.WARNING):
    """Set up logging configuration for the AnkiLogger addon.

    Creates a rotating file handler that logs to anki_logger.log in the addon directory.
    The log file is limited to 5MB with 3 backup files.

    Args:
        level: The logging level (default: WARNING - shows warnings and errors)
    """
    global _configured

    if _configured:
        return

    # Create logger
    logger = logging.getLogger('AnkiLogger')
    logger.setLevel(level)

    # Create rotating file handler (5MB per file, keep 3 backups)
    handler = RotatingFileHandler(
        LOG_FILE,
        maxBytes=5 * 1024 * 1024,  # 5MB
        backupCount=3,
        encoding='utf-8'
    )
    handler.setLevel(level)

    # Create formatter with detailed information
    formatter = logging.Formatter(
        '%(asctime)s - %(name)s - %(levelname)s - %(filename)s:%(lineno)d - %(message)s',
        datefmt='%Y-%m-%d %H:%M:%S'
    )
    handler.setFormatter(formatter)

    # Add handler to logger
    logger.addHandler(handler)

    _configured = True
    logger.info("=" * 80)
    logger.info("AnkiLogger logging initialized")
    logger.info(f"Log file: {LOG_FILE}")
    logger.info("=" * 80)


def get_logger(name):
    """Get a logger instance for a specific module.

    Args:
        name: The name of the module (typically __name__)

    Returns:
        A configured logger instance
    """
    if not _configured:
        setup_logging()

    return logging.getLogger(f'AnkiLogger.{name}')


# Initialize logging on import
setup_logging()
