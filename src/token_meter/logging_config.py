import logging
from logging.handlers import RotatingFileHandler
from pathlib import Path

LOG_PATH = Path.home() / ".cache" / "token_meter.log"
LOG_PATH.parent.mkdir(parents=True, exist_ok=True)

# Basic formatter
_formatter = logging.Formatter(
    "%(asctime)s %(levelname)s %(name)s %(module)s:%(lineno)d - %(message)s"
)

# Rotating file handler
_handler = RotatingFileHandler(LOG_PATH, maxBytes=1_000_000, backupCount=3)
_handler.setFormatter(_formatter)

# Root logger for the package
_logger = logging.getLogger("token_meter")
if not _logger.handlers:
    _logger.addHandler(_handler)
    _logger.setLevel(logging.INFO)


def get_logger(name: str | None = None) -> logging.Logger:
    """Return a logger scoped under the token_meter package.

    Use get_logger(__name__) to obtain a module-specific logger.
    """
    if name:
        return logging.getLogger(f"token_meter.{name}")
    return _logger
