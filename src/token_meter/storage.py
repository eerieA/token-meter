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
