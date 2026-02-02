from dataclasses import dataclass
from datetime import datetime
from decimal import Decimal


@dataclass
class UsageRecord:
    provider: str
    timestamp: datetime
    tokens_input: int | None
    tokens_output: int | None
    # Decimal for monetary amounts to avoid float rounding issues
    cost_usd: Decimal
    model: str | None = None
