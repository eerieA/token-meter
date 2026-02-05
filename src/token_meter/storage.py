import json
from pathlib import Path
from datetime import datetime, timedelta, timezone
from token_meter.domain import UsageRecord
from token_meter.logger import get_logger

logger = get_logger(__name__)


CACHE_PATH = Path.home() / ".token-meter" / "api_usage.json"


def load_cache() -> dict:
    try:
        if not CACHE_PATH.exists():
            logger.info("Cache file does not exist: %s", CACHE_PATH)
            return {}
        text = CACHE_PATH.read_text()
        data = json.loads(text)
        logger.info("Loaded cache from %s", CACHE_PATH)
        return data
    except Exception as e:
        logger.exception("Failed to load cache: %s", e)
        return {}


def save_cache(data: dict):
    try:
        CACHE_PATH.parent.mkdir(parents=True, exist_ok=True)
        CACHE_PATH.write_text(json.dumps(data))
        logger.info("Saved cache to %s", CACHE_PATH)
    except Exception as e:
        logger.exception("Failed to write cache to %s: %s", CACHE_PATH, e)


def cache_valid(last_fetch: str, ttl_seconds: int) -> bool:
    last_fetch_dt = datetime.fromisoformat(last_fetch)

    # Normalize in case older cache entries were naive
    if last_fetch_dt.tzinfo is None:
        last_fetch_dt = last_fetch_dt.replace(tzinfo=timezone.utc)

    now_dt = datetime.now(timezone.utc)

    return (now_dt - last_fetch_dt).total_seconds() < ttl_seconds


# Baseline persistence helpers
def load_baseline() -> dict | None:
    """Return the saved baseline dict or None if not present.

    Format: { "amount": "123.45", "start": "2024-01-01T00:00:00+00:00" }
    """
    try:
        data = load_cache()
        b = data.get("baseline")
        if not b:
            logger.info("No baseline configured in cache")
            return None
        return b
    except Exception as e:
        logger.exception("Failed to load baseline: %s", e)
        return None


def save_baseline(amount_str: str, start_iso: str):
    """Save the baseline amount and ISO datetime into the cache file."""
    try:
        data = load_cache()
        data["baseline"] = {"amount": str(amount_str), "start": str(start_iso)}
        save_cache(data)
        logger.info("Saved baseline to cache: %s %s", amount_str, start_iso)
    except Exception as e:
        logger.exception("Failed to save baseline: %s", e)


def clear_baseline():
    """Remove any saved baseline from the cache."""
    try:
        data = load_cache()
        if "baseline" in data:
            data.pop("baseline")
            save_cache(data)
            logger.info("Cleared baseline from cache")
    except Exception as e:
        logger.exception("Failed to clear baseline: %s", e)

