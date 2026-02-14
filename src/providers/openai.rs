use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;
use rust_decimal::Decimal;
use std::time::Duration;

const OPENAI_API_BASE: &str = "https://api.openai.com/v1";
const MAX_RETRIES: usize = 3;

#[derive(Debug, Deserialize)]
struct AmountValue {
    value: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ResultEntry {
    amount: AmountValue,
}

#[derive(Debug, Deserialize)]
struct Bucket {
    start_time: i64,
    results: Vec<ResultEntry>,
}

#[derive(Debug, Deserialize)]
struct CostsPage {
    has_more: Option<bool>,
    data: Option<Vec<Bucket>>,
    next_page: Option<String>,
}

pub struct OpenAIProvider {
    client: Client,
    api_key: String,
}

impl OpenAIProvider {
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
        let mut params: Vec<(String, String)> = vec![
            ("start_time".to_string(), start_time.to_string()),
            ("bucket_width".to_string(), "1d".to_string()),
            ("limit".to_string(), "180".to_string()),
        ];
        if let Some(e) = end_time {
            params.push(("end_time".to_string(), e.to_string()));
        }

        let mut page: Option<String> = None;
        let mut amounts: Vec<Decimal> = Vec::new();

        loop {
            if let Some(ref p) = page {
                params.retain(|(k, _)| k.as_str() != "page");
                params.push(("page".to_string(), p.clone()));
            }

            let url = format!("{}/organization/costs", OPENAI_API_BASE);

            let resp = self
                .do_get_with_retries(&url, &params)
                .await
                .context("requesting OpenAI costs")?;

            let page_obj: CostsPage = serde_json::from_value(resp)
                .context("parsing OpenAI costs response")?;

            // Compute per-page sums and counts for debug visibility
            if let Some(buckets) = page_obj.data {
                let mut page_sum = Decimal::ZERO;
                let page_count = buckets.len();
                for bucket in &buckets {
                    let mut bucket_sum = Decimal::ZERO;
                    for r in &bucket.results {
                        // amount.value may be a number or string; normalize to string then Decimal
                        let val = match &r.amount.value {
                            serde_json::Value::String(s) => s.clone(),
                            serde_json::Value::Number(n) => n.to_string(),
                            other => other.to_string(),
                        };
                        match val.parse::<Decimal>() {
                            Ok(d) => {
                                page_sum += d;
                                bucket_sum += d;
                                amounts.push(d);
                            },
                            Err(e) => tracing::warn!("Failed to parse amount value '{}' : {}", val, e),
                        }
                    }
                    eprintln!("[openai] bucket: start_time={} bucket_sum={} results={}", bucket.start_time, bucket_sum, bucket.results.len());
                }
                eprintln!("[openai] page fetched: buckets={} page_sum={} has_more={} next_page={:?}", page_count, page_sum, page_obj.has_more.unwrap_or(false), page_obj.next_page);
            } else {
                eprintln!("[openai] page fetched: no buckets in response");
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
        let mut attempt = 0usize;
        loop {
            attempt += 1;
            let mut req = self.client.get(url);
            req = req.header("Authorization", format!("Bearer {}", self.api_key));
            req = req.header("Content-Type", "application/json");
            // attach params
            for (k, v) in params.iter() {
                req = req.query(&[(k.as_str(), v.as_str())]);
            }
            // Debug: log request attempt and parameters so we can confirm network activity
            eprintln!("[openai] GET {} params={:?} attempt={}", url, params, attempt);
            let resp = req.send().await;
            match resp {
                Ok(r) => {
                    let status = r.status();
                    let txt = r.text().await.unwrap_or_else(|_| "".to_string());
                    // Debug: show response status and a short snippet of the body
                    let snippet = if txt.len() > 200 { &txt[..200] } else { &txt[..] };
                    eprintln!("[openai] response status={} body_len={} snippet={}", status, txt.len(), snippet);
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
                        eprintln!("[openai] max retries reached, returning HTTP error {}", status);
                        return Err(anyhow!("HTTP error {}: {}", status, txt));
                    } else {
                        eprintln!("[openai] HTTP error {} returned", status);
                        return Err(anyhow!("HTTP error {}: {}", status, txt));
                    }
                }
                Err(e) => {
                    eprintln!("[openai] network error on attempt {}: {}", attempt, e);
                    if attempt < MAX_RETRIES {
                        let backoff = Duration::from_secs(1 << (attempt - 1));
                        tracing::warn!("Network error: {} - retrying in {:?}", e, backoff);
                        tokio::time::sleep(backoff).await;
                        continue;
                    }
                    eprintln!("[openai] network error: giving up: {}", e);
                    return Err(anyhow!(e));
                }
            }
        }
    }
}
