from datetime import datetime
from token_meter.providers.openai import OpenAIProvider
from token_meter.storage import load_cache, save_cache, cache_valid

class UsageAggregator:
    def __init__(self, openai_key: str):
        self.openai = OpenAIProvider(openai_key)

    def fetch(self):
        cache = load_cache()
        now = datetime.utcnow().isoformat()

        if "openai" in cache and cache_valid(cache["openai"]["fetched_at"], 300):
            return cache["openai"]["data"]

        # Fetch costs for today (UTC)
        start_of_day = datetime.utcnow().replace(hour=0, minute=0, second=0, microsecond=0)
        start_time = int(start_of_day.timestamp())

        records = self.openai.fetch_costs(start_time)
        total_cost = sum(r.cost_usd for r in records)

        cache["openai"] = {
            "fetched_at": now,
            "data": total_cost
        }
        save_cache(cache)

        return total_cost

