import asyncio
from typing import Any, Callable, Optional

import httpx
from datetime import datetime, timezone
from decimal import Decimal
from token_meter.domain import UsageRecord
from token_meter.logger import get_logger
from PySide6.QtCore import QObject, Signal


OPENAI_API_BASE = "https://api.openai.com/v1"

# Retry configuration
_MAX_RETRIES = 3
_BACKOFF_FACTOR = 1  # base seconds

logger = get_logger(__name__)


class OpenAIProviderError(Exception):
    def __init__(
        self, message: str, status: int | None = None, body: str | None = None
    ):
        super().__init__(message)
        self.status = status
        self.body = body


class AsyncOpenAIProvider:
    """Async provider for OpenAI organization costs using httpx.AsyncClient.

    This mirrors the behavior of the previous synchronous OpenAIProvider but
    performs network I/O using asyncio-friendly primitives. It supports an
    optional retry_callback(attempt:int, wait_seconds:float) which will be
    invoked before sleeping on a retry; this is used by callers that want
    to expose retry progress to the UI.
    """

    def __init__(self, admin_api_key: str):
        # IMPORTANT: this must be an *admin* key
        self.api_key = admin_api_key

    async def fetch_costs(
        self,
        start_time: int,
        end_time: int | None = None,
        project_ids: list[str] | None = None,
        paginate: bool = True,
        retry_callback: Optional[Callable[[int, float], None]] = None,
    ) -> list[UsageRecord]:
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json",
        }

        params: dict[str, Any] = {
            "start_time": start_time,
            "bucket_width": "1d",
            "limit": 180,
        }

        if end_time is not None:
            params["end_time"] = end_time

        if project_ids:
            params["project_ids"] = project_ids

        # If paginate is False, do a single request and return the first page
        if not paginate:
            payload = await self._async_get(
                f"{OPENAI_API_BASE}/organization/costs", headers, params, retry_callback
            )
            return self._normalize_costs(payload)

        # Otherwise iterate through pages
        records: list[UsageRecord] = []
        page: str | None = None

        while True:
            if page:
                params["page"] = page

            payload = await self._async_get(
                f"{OPENAI_API_BASE}/organization/costs", headers, params, retry_callback
            )
            records.extend(self._normalize_costs(payload))

            if not payload.get("has_more"):
                break

            page = payload.get("next_page")

        return records

    async def _async_get(
        self,
        url: str,
        headers: dict,
        params: dict,
        retry_callback: Optional[Callable[[int, float], None]] = None,
    ) -> dict:
        """Perform an async GET with retries and exponential backoff.

        Raises OpenAIProviderError for non-retriable HTTP errors or on final failure.
        """
        for attempt in range(1, _MAX_RETRIES + 1):
            try:
                logger.info(
                    "Requesting %s (attempt=%d) params=%s", url, attempt, params
                )
                async with httpx.AsyncClient(timeout=15) as client:
                    r = await client.get(url, headers=headers, params=params)
            except httpx.RequestError as e:
                # Network-level error
                logger.warning("Network error on attempt %d: %s", attempt, e)
                if attempt < _MAX_RETRIES:
                    backoff = _BACKOFF_FACTOR * (2 ** (attempt - 1))
                    if retry_callback:
                        try:
                            retry_callback(attempt, backoff)
                        except Exception:
                            pass
                    logger.info("Sleeping for %s seconds before retry", backoff)
                    await asyncio.sleep(backoff)
                    continue
                raise OpenAIProviderError(
                    "Network error while requesting OpenAI", None, str(e)
                ) from e

            # Got a response; handle status codes
            status = r.status_code
            body = None
            try:
                # httpx Response.text is populated for async clients
                body = r.text
            except Exception:
                body = "<unavailable>"

            if status >= 400:
                # Treat 429 and 5xx as transient
                if status == 429 or status >= 500:
                    logger.warning(
                        "Transient HTTP error %s on attempt %d: %s",
                        status,
                        attempt,
                        body,
                    )
                    if attempt < _MAX_RETRIES:
                        backoff = _BACKOFF_FACTOR * (2 ** (attempt - 1))
                        if retry_callback:
                            try:
                                retry_callback(attempt, backoff)
                            except Exception:
                                pass
                        logger.info("Sleeping for %s seconds before retry", backoff)
                        await asyncio.sleep(backoff)
                        continue
                    raise OpenAIProviderError(f"HTTP error {status}", status, body)
                else:
                    # Other 4xx errors are considered permanent
                    logger.error("Non-retriable HTTP error %s: %s", status, body)
                    raise OpenAIProviderError(f"HTTP error {status}", status, body)

            # OK
            try:
                payload = r.json()
            except Exception as e:
                logger.error("Failed to parse JSON response: %s", e)
                raise OpenAIProviderError("Invalid JSON response", status, body) from e

            logger.info(
                "Received page: has_more=%s, entries=%d",
                payload.get("has_more"),
                len(payload.get("data", [])),
            )
            return payload

        # Shouldn't reach here
        raise OpenAIProviderError("Exceeded retries when requesting OpenAI")

    def _normalize_costs(self, payload) -> list[UsageRecord]:
        """
        Normalize OpenAI cost buckets into UsageRecord entries.
        """
        records: list[UsageRecord] = []

        for bucket in payload.get("data", []):
            bucket_start = bucket["start_time"]

            ts = datetime.fromtimestamp(bucket_start, tz=timezone.utc)

            for result in bucket.get("results", []):
                # The API returns amounts as strings or numbers. Use Decimal for money
                amount = result["amount"]["value"]

                amount_dec = Decimal(str(amount))

                records.append(
                    UsageRecord(
                        provider="openai",
                        timestamp=ts,
                        tokens_input=None,
                        tokens_output=None,
                        cost_usd=amount_dec,
                        model=None,  # cost endpoint is model-agnostic
                    )
                )

        return records


class AsyncFetcher(QObject):
    """A QObject-friendly async fetcher that runs on the app's asyncio event loop.

    It emits Qt signals for started/retry/success/failure/finished so UI code can
    react without blocking the main thread.
    """

    fetching_started = Signal()
    fetching_retry = Signal(int, float)  # attempt, wait_seconds
    fetching_succeeded = Signal(object, object)  # total: Decimal, raw_records: list[UsageRecord]
    fetching_failed = Signal(object, object, object)  # exc, status, body
    fetching_finished = Signal()

    def __init__(self, admin_api_key: str, cache_ttl: int = 300, parent=None):
        super().__init__(parent)
        self._provider = AsyncOpenAIProvider(admin_api_key)
        self.cache_ttl = cache_ttl
        self._task: Optional[asyncio.Task] = None

    async def _run_fetch(self):
        self.fetching_started.emit()
        try:
            # month-to-date start at UTC month start
            now = datetime.now(timezone.utc)
            start = datetime(now.year, now.month, 1, tzinfo=timezone.utc)
            start_ts = int(start.timestamp())

            def _retry_callback(attempt: int, wait: float) -> None:
                try:
                    # emit signal; since we run on the Qt event loop (qasync), this is safe
                    self.fetching_retry.emit(attempt, wait)
                except Exception:
                    pass

            records = await self._provider.fetch_costs(start_ts, retry_callback=_retry_callback)

            total = sum((r.cost_usd for r in records), Decimal(0))

            # Emit success with total and the raw records list
            self.fetching_succeeded.emit(total, records)
        except asyncio.CancelledError:
            # task was cancelled, treat as finished without emitting failed
            pass
        except OpenAIProviderError as exc:
            self.fetching_failed.emit(exc, exc.status, exc.body)
        except Exception as exc:
            status = getattr(exc, "status", None)
            body = getattr(exc, "body", None)
            self.fetching_failed.emit(exc, status, body)
        finally:
            self.fetching_finished.emit()

    def start(self):
        if self._task and not self._task.done():
            return
        self._task = asyncio.create_task(self._run_fetch())

    def cancel(self):
        if self._task and not self._task.done():
            self._task.cancel()


