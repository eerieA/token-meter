from dataclasses import dataclass
from datetime import datetime

@dataclass
class UsageRecord:
    provider: str
    timestamp: datetime
    tokens_input: int | None
    tokens_output: int | None
    cost_usd: float
    model: str | None = None

