use anyhow::{Result, Context};
use rust_decimal::Decimal;
use chrono::{Utc, TimeZone, Datelike};
use crate::providers::openai::OpenAIProvider;
use crate::providers::anthropic::AnthropicProvider;

pub struct UsageAggregator {
    openai: Option<OpenAIProvider>,
    anthropic: Option<AnthropicProvider>,
}

impl UsageAggregator {
    pub fn new_openai(api_key: &str) -> Self {
        Self {
            openai: Some(OpenAIProvider::new(api_key.to_string())),
            anthropic: None,
        }
    }

    pub fn new_anthropic(api_key: &str) -> Self {
        Self {
            openai: None,
            anthropic: Some(AnthropicProvider::new(api_key.to_string())),
        }
    }

    pub async fn fetch_month_to_date(&self) -> Result<Decimal> {
        let provider = self.openai.as_ref().context("OpenAI provider not configured")?;
        let start_ts = start_of_current_month();
        eprintln!("[aggregator] fetch_month_to_date (openai): start_ts={}", start_ts);
        let amounts = provider.fetch_costs(start_ts, None, true).await.context("fetching costs from OpenAI provider")?;
        Ok(sum(amounts))
    }

    pub async fn fetch_since(&self, start_ts: i64, end_ts: Option<i64>) -> Result<Decimal> {
        let provider = self.openai.as_ref().context("OpenAI provider not configured")?;
        eprintln!("[aggregator] fetch_since (openai): start_ts={} end_ts={:?}", start_ts, end_ts);
        let amounts = provider.fetch_costs(start_ts, end_ts, true).await.context("fetching costs since")?;
        Ok(sum(amounts))
    }

    pub async fn fetch_month_to_date_anthropic(&self) -> Result<Decimal> {
        let provider = self.anthropic.as_ref().context("Anthropic provider not configured")?;
        let start_ts = start_of_current_month();
        eprintln!("[aggregator] fetch_month_to_date (anthropic): start_ts={}", start_ts);
        let amounts = provider.fetch_costs(start_ts, None, true).await.context("fetching costs from Anthropic provider")?;
        Ok(sum(amounts))
    }

    pub async fn fetch_since_anthropic(&self, start_ts: i64, end_ts: Option<i64>) -> Result<Decimal> {
        let provider = self.anthropic.as_ref().context("Anthropic provider not configured")?;
        eprintln!("[aggregator] fetch_since (anthropic): start_ts={} end_ts={:?}", start_ts, end_ts);
        let amounts = provider.fetch_costs(start_ts, end_ts, true).await.context("fetching anthropic costs since")?;
        Ok(sum(amounts))
    }
}

fn start_of_current_month() -> i64 {
    let now = Utc::now();
    Utc.with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0).unwrap().timestamp()
}

fn sum(amounts: Vec<Decimal>) -> Decimal {
    let total = amounts.iter().fold(Decimal::ZERO, |acc, a| acc + a);
    eprintln!("[aggregator] sum: {} buckets, total={}", amounts.len(), total);
    total
}
