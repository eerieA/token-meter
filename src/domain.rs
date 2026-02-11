use rust_decimal::Decimal;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct UsageRecord {
    pub provider: String,
    pub timestamp: DateTime<Utc>,
    pub cost_usd: Decimal,
}
