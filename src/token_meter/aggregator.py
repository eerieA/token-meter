from datetime import datetime, timezone
from decimal import Decimal
from token_meter.providers.openai import OpenAIProvider
from token_meter.storage import load_cache, save_cache, cache_valid


class UsageAggregator:
    def __init__(self, openai_key: str):
        self.openai = OpenAIProvider(openai_key)

    def fetch(self):
        cache = load_cache()
        # fetched_at should be an ISO string to be json serializable
        now_dt = datetime.now(timezone.utc)
        fetched_at = now_dt.isoformat()

        # If have a recent cached total, return it
        if "openai" in cache and cache_valid(cache["openai"]["fetched_at"], 300):
            cached = cache["openai"]["data"]
            # it should have stored totals as strings
            try:
                return Decimal(str(cached))
            except Exception:
                # If cache is malformed, fall back to fetching
                pass

        # Start of current month in UTC
        start_of_month = now_dt.replace(
            day=1, hour=0, minute=0, second=0, microsecond=0
        )
        start_time = int(start_of_month.timestamp())

        # Fetch all pages and aggregate using Decimal
        records = self.openai.fetch_costs(start_time, paginate=True)
        total_cost = sum((r.cost_usd for r in records), Decimal("0"))

        cache["openai"] = {"fetched_at": fetched_at, "data": str(total_cost)}
        save_cache(cache)

        return total_cost
