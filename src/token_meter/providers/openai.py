import time
from typing import Any

import requests
from datetime import datetime, timezone
from decimal import Decimal
from token_meter.domain import UsageRecord
from token_meter.logger import get_logger


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


class OpenAIProvider:
    def __init__(self, admin_api_key: str):
        # IMPORTANT: this must be an *admin* key
        self.api_key = admin_api_key

    def fetch_costs(
        self,
        start_time: int,
        end_time: int | None = None,
        project_ids: list[str] | None = None,
        paginate: bool = True,
    ) -> list[UsageRecord]:
        """
        Fetch authoritative cost data from OpenAI.

        start_time / end_time: Unix seconds (UTC)
        paginate: if False, only fetch the first page returned by the endpoint.
        """
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
            payload = self._do_get(
                f"{OPENAI_API_BASE}/organization/costs", headers, params
            )
            return self._normalize_costs(payload)

        # Otherwise iterate through pages
        records: list[UsageRecord] = []
        page: str | None = None

        while True:
            if page:
                params["page"] = page

            payload = self._do_get(
                f"{OPENAI_API_BASE}/organization/costs", headers, params
            )
            records.extend(self._normalize_costs(payload))

            if not payload.get("has_more"):
                break

            page = payload.get("next_page")

        return records

    def _do_get(self, url: str, headers: dict, params: dict) -> dict:
        """Perform a GET with retries and exponential backoff.

        Raises OpenAIProviderError for non-retriable HTTP errors or on final failure.
        """
        for attempt in range(1, _MAX_RETRIES + 1):
            try:
                logger.info(
                    "Requesting %s (attempt=%d) params=%s", url, attempt, params
                )
                r = requests.get(url, headers=headers, params=params, timeout=15)
            except requests.RequestException as e:
                # Network-level error
                logger.warning("Network error on attempt %d: %s", attempt, e)
                if attempt < _MAX_RETRIES:
                    backoff = _BACKOFF_FACTOR * (2 ** (attempt - 1))
                    logger.info("Sleeping for %s seconds before retry", backoff)
                    time.sleep(backoff)
                    continue
                raise OpenAIProviderError(
                    "Network error while requesting OpenAI", None, str(e)
                ) from e

            # Got a response; handle status codes
            status = r.status_code
            body = None
            try:
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
                        logger.info("Sleeping for %s seconds before retry", backoff)
                        time.sleep(backoff)
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
