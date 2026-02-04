from datetime import datetime, timezone
from decimal import Decimal
from token_meter.providers.openai import AsyncOpenAIProvider, OpenAIProviderError
from token_meter.storage import load_cache, save_cache, cache_valid
from token_meter.logger import get_logger

logger = get_logger(__name__)


class UsageAggregator:
    def __init__(self, openai_key: str):
        self.openai = AsyncOpenAIProvider(openai_key)

    async def fetch(self) -> Decimal:
        cache = load_cache()
        # fetched_at should be an ISO string to be json serializable
        now_dt = datetime.now(timezone.utc)
        fetched_at = now_dt.isoformat()

        # If have a recent cached total, return it
        if "openai" in cache and cache_valid(cache["openai"]["fetched_at"], 300):
            cached = cache["openai"]["data"]
            try:
                value = Decimal(str(cached))
                logger.info("Cache hit: returning cached total %s", value)
                return value
            except Exception:
                logger.warning("Malformed cache entry, refetching")

        # Start of current month in UTC
        start_of_month = now_dt.replace(
            day=1, hour=0, minute=0, second=0, microsecond=0
        )
        start_time = int(start_of_month.timestamp())

        logger.info("Fetching costs from OpenAI starting at %s", start_time)

        try:
            records = await self.openai.fetch_costs(start_time, paginate=True)
        except OpenAIProviderError as e:
            logger.exception(
                "Failed to fetch costs: %s (status=%s)", e, getattr(e, "status", None)
            )
            raise

        total_cost = sum((r.cost_usd for r in records), Decimal("0"))

        logger.info(
            "Aggregated total cost: %s from %d records", total_cost, len(records)
        )

        cache["openai"] = {"fetched_at": fetched_at, "data": str(total_cost)}
        save_cache(cache)

        return total_cost
