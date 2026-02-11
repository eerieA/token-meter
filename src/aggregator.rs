use anyhow::{Result, Context};
use rust_decimal::Decimal;
use chrono::{Utc, TimeZone, Datelike};
use crate::providers::openai::OpenAIProvider;

pub struct UsageAggregator {
    provider: OpenAIProvider,
}

impl UsageAggregator {
    pub fn new(api_key: &str) -> Self {
        Self {
            provider: OpenAIProvider::new(api_key.to_string()),
        }
    }

    pub async fn fetch_month_to_date(&self) -> Result<Decimal> {
        let now = Utc::now();
        let start_of_month = Utc.ymd_opt(now.year(), now.month(), 1).single().unwrap().and_hms_opt(0, 0, 0).unwrap();
        let start_ts = start_of_month.timestamp();
        let amounts = self
            .provider
            .fetch_costs(start_ts, None, true)
            .await
            .context("fetching costs from provider")?;

        let mut total = Decimal::ZERO;
        for a in amounts {
            total += a;
        }
        Ok(total)
    }

    pub async fn fetch_since(&self, start_ts: i64, end_ts: Option<i64>) -> Result<Decimal> {
        let amounts = self
            .provider
            .fetch_costs(start_ts, end_ts, true)
            .await
            .context("fetching costs since")?;
        let mut total = Decimal::ZERO;
        for a in amounts {
            total += a;
        }
        Ok(total)
    }
}
