from datetime import datetime
from token_meter.providers.openai import OpenAIProvider
from token_meter.storage import load_cache, save_cache, cache_valid


class UsageAggregator:
    def __init__(self, openai_key: str):
        self.openai = OpenAIProvider(openai_key)

    def fetch(self):
        cache = load_cache()
        # fetched_at should be an ISO string to be json serializable
        now_dt = datetime.utcnow()
        fetched_at = now_dt.isoformat()

        if "openai" in cache and cache_valid(cache["openai"]["fetched_at"], 300):
            return cache["openai"]["data"]

        # Fetch costs for the start of the current month
        start_of_month = now_dt.replace(
            day=1, hour=0, minute=0, second=0, microsecond=0
        )
        start_time = int(start_of_month.timestamp())

        # Only fetch the first page for now (no pagination)
        records = self.openai.fetch_costs(start_time, paginate=False)
        total_cost = sum(r.cost_usd for r in records)

        cache["openai"] = {"fetched_at": fetched_at, "data": total_cost}
        save_cache(cache)

        return total_cost
