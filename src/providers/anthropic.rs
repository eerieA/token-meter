use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;
use rust_decimal::Decimal;
use std::time::Duration;
use chrono::{DateTime, Utc, TimeZone};

const ANTHROPIC_API_BASE: &str = "https://api.anthropic.com/v1";
const ANTHROPIC_COST_REPORT: &str = "/organizations/cost_report";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_RETRIES: usize = 3;

#[derive(Debug, Deserialize)]
struct ResultEntry {
    amount: String,
}

#[derive(Debug, Deserialize)]
struct Bucket {
    starting_at: String,
    results: Vec<ResultEntry>,
}

#[derive(Debug, Deserialize)]
struct CostReport {
    has_more: Option<bool>,
    data: Option<Vec<Bucket>>,
    next_page: Option<String>,
}

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
}

impl AnthropicProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
        }
    }

    pub async fn fetch_costs(
        &self,
        start_time: i64,
        end_time: Option<i64>,
        paginate: bool,
    ) -> Result<Vec<Decimal>> {
        // Anthropic requires ending_at to be in the past and the range must span at least one
        // completed daily bucket. Use start of today (UTC) as the upper bound — this covers all
        // completed days. If starting_at >= start of today (no completed buckets yet), skip the
        // network call and return empty immediately.
        let today_start = start_of_today();
        if start_time >= today_start {
            eprintln!("[anthropic] no completed daily buckets yet (start_time >= today), returning empty");
            return Ok(vec![]);
        }
        let starting_at = unix_to_rfc3339(start_time);
        let ending_at = end_time
            .map(unix_to_rfc3339)
            .unwrap_or_else(|| unix_to_rfc3339(today_start));
        let mut params: Vec<(String, String)> = vec![
            ("starting_at".to_string(), starting_at),
            ("ending_at".to_string(), ending_at),
            ("bucket_width".to_string(), "1d".to_string()),
        ];

        let mut page: Option<String> = None;
        let mut amounts: Vec<Decimal> = Vec::new();

        loop {
            if let Some(ref p) = page {
                params.retain(|(k, _)| k.as_str() != "page");
                params.push(("page".to_string(), p.clone()));
            }

            let url = format!("{}{}", ANTHROPIC_API_BASE, ANTHROPIC_COST_REPORT);

            let resp = self
                .do_get_with_retries(&url, &params)
                .await
                .context("requesting Anthropic cost report")?;

            let page_obj: CostReport = serde_json::from_value(resp)
                .context("parsing Anthropic cost report response")?;

            if let Some(buckets) = page_obj.data {
                let mut page_sum = Decimal::ZERO;
                let page_count = buckets.len();
                for bucket in &buckets {
                    let mut bucket_sum = Decimal::ZERO;
                    for r in &bucket.results {
                        match r.amount.parse::<Decimal>() {
                            Ok(d) => {
                                page_sum += d;
                                bucket_sum += d;
                                amounts.push(d);
                            }
                            Err(e) => tracing::warn!("Failed to parse Anthropic amount '{}': {}", r.amount, e),
                        }
                    }
                    eprintln!("[anthropic] bucket: starting_at={} bucket_sum={} results={}", bucket.starting_at, bucket_sum, bucket.results.len());
                }
                eprintln!("[anthropic] page fetched: buckets={} page_sum={} has_more={} next_page={:?}", page_count, page_sum, page_obj.has_more.unwrap_or(false), page_obj.next_page);
            } else {
                eprintln!("[anthropic] page fetched: no buckets in response");
            }

            if !paginate {
                break;
            }

            if !page_obj.has_more.unwrap_or(false) {
                break;
            }

            page = page_obj.next_page;
            if page.is_none() {
                break;
            }
        }

        Ok(amounts)
    }

    async fn do_get_with_retries(&self, url: &str, params: &[(String, String)]) -> Result<serde_json::Value> {
        // Build query string manually to avoid percent-encoding colons in RFC 3339 timestamps.
        // reqwest's .query() uses form_urlencoded which encodes ':' → '%3A', which Anthropic rejects.
        let query: String = params.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        let full_url = format!("{}?{}", url, query);

        let mut attempt = 0usize;
        loop {
            attempt += 1;
            let mut req = self.client.get(&full_url);
            req = req.header("X-Api-Key", &self.api_key);
            req = req.header("anthropic-version", ANTHROPIC_VERSION);
            req = req.header("Content-Type", "application/json");
            eprintln!("[anthropic] GET {} params={:?} attempt={}", full_url, params, attempt);
            let resp = req.send().await;
            match resp {
                Ok(r) => {
                    let status = r.status();
                    let txt = r.text().await.unwrap_or_else(|_| "".to_string());
                    let snippet = if txt.len() > 200 { &txt[..200] } else { &txt[..] };
                    eprintln!("[anthropic] response status={} body_len={} snippet={}", status, txt.len(), snippet);
                    if status.is_success() {
                        let j: serde_json::Value = serde_json::from_str(&txt).context("invalid json")?;
                        return Ok(j);
                    } else if status.as_u16() == 429 || status.is_server_error() {
                        if attempt < MAX_RETRIES {
                            let backoff = Duration::from_secs(1 << (attempt - 1));
                            tracing::warn!("Transient HTTP {} - retrying in {:?}", status, backoff);
                            tokio::time::sleep(backoff).await;
                            continue;
                        }
                        eprintln!("[anthropic] max retries reached, returning HTTP error {}", status);
                        return Err(anyhow!("HTTP error {}: {}", status, txt));
                    } else {
                        eprintln!("[anthropic] HTTP error {} returned", status);
                        return Err(anyhow!("HTTP error {}: {}", status, txt));
                    }
                }
                Err(e) => {
                    eprintln!("[anthropic] network error on attempt {}: {}", attempt, e);
                    if attempt < MAX_RETRIES {
                        let backoff = Duration::from_secs(1 << (attempt - 1));
                        tracing::warn!("Network error: {} - retrying in {:?}", e, backoff);
                        tokio::time::sleep(backoff).await;
                        continue;
                    }
                    eprintln!("[anthropic] network error: giving up: {}", e);
                    return Err(anyhow!(e));
                }
            }
        }
    }
}

fn unix_to_rfc3339(ts: i64) -> String {
    // Use Z suffix (not +00:00) to avoid percent-encoding issues in URL query params
    Utc.timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(|| DateTime::<Utc>::from(std::time::UNIX_EPOCH))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

fn start_of_today() -> i64 {
    use chrono::Datelike;
    let now = Utc::now();
    Utc.with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .unwrap()
        .timestamp()
}

