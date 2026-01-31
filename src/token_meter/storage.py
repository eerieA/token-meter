import json
from pathlib import Path
from datetime import datetime, timedelta
from token_meter.domain import UsageRecord

CACHE_PATH = Path.home() / ".cache" / "ai_usage.json"

def load_cache() -> dict:
    if not CACHE_PATH.exists():
        return {}
    return json.loads(CACHE_PATH.read_text())

def save_cache(data: dict):
    CACHE_PATH.parent.mkdir(parents=True, exist_ok=True)
    CACHE_PATH.write_text(json.dumps(data))

def cache_valid(last_fetch: str, ttl_seconds: int) -> bool:
    return (
        datetime.utcnow() -
        datetime.fromisoformat(last_fetch)
    ) < timedelta(seconds=ttl_seconds)
