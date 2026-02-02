import requests
from datetime import datetime, timezone
from token_meter.domain import UsageRecord

OPENAI_API_BASE = "https://api.openai.com/v1"


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

        params = {
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
            r = requests.get(
                f"{OPENAI_API_BASE}/organization/costs",
                headers=headers,
                params=params,
                timeout=15,
            )
            r.raise_for_status()
            payload = r.json()
            return self._normalize_costs(payload)

        # Otherwise iterate through pages
        records: list[UsageRecord] = []
        page: str | None = None

        while True:
            if page:
                params["page"] = page

            r = requests.get(
                f"{OPENAI_API_BASE}/organization/costs",
                headers=headers,
                params=params,
                timeout=15,
            )
            r.raise_for_status()

            payload = r.json()
            records.extend(self._normalize_costs(payload))

            if not payload.get("has_more"):
                break

            page = payload.get("next_page")

        return records

    def _normalize_costs(self, payload) -> list[UsageRecord]:
        """
        Normalize OpenAI cost buckets into UsageRecord entries.
        """
        records: list[UsageRecord] = []

        for bucket in payload.get("data", []):
            bucket_start = bucket["start_time"]

            ts = datetime.fromtimestamp(bucket_start, tz=timezone.utc)

            for result in bucket.get("results", []):
                amount = result["amount"]["value"]

                records.append(
                    UsageRecord(
                        provider="openai",
                        timestamp=ts,
                        tokens_input=None,
                        tokens_output=None,
                        cost_usd=float(amount),
                        model=None,  # cost endpoint is model-agnostic
                    )
                )

        return records
