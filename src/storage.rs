use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use anyhow::{Result, Context};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheData {
    pub openai_total: Option<String>,
    pub fetched_at: Option<String>,
    pub baseline: Option<Baseline>,
    pub baseline_used: Option<String>,
    pub baseline_remaining: Option<String>,
    // Anthropic equivalents
    pub anthropic_total: Option<String>,
    pub anthropic_fetched_at: Option<String>,
    pub anthropic_baseline: Option<Baseline>,
    pub anthropic_baseline_used: Option<String>,
    pub anthropic_baseline_remaining: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Baseline {
    pub amount: String,
    pub start_iso: String,
}

fn base_dir() -> Option<PathBuf> {
    if let Some(home) = env::home_dir() {
        Some(home.join(".token-meter"))
    } else {
        None
    }
}

fn cache_path() -> Option<PathBuf> {
    let mut p = base_dir()?;
    p.push("api_usage.json");
    Some(p)
}

fn credentials_path() -> Option<PathBuf> {
    let mut p = base_dir()?;
    p.push("credentials.json");
    Some(p)
}

fn load_credentials_json() -> serde_json::Value {
    if let Some(p) = credentials_path() {
        if let Ok(txt) = fs::read_to_string(&p) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&txt) {
                return v;
            }
        }
    }
    serde_json::json!({})
}

fn save_credentials_json(v: &serde_json::Value) -> Result<()> {
    let mut p = base_dir().context("cannot find config dir")?;
    fs::create_dir_all(&p).ok();
    p.push("credentials.json");
    fs::write(&p, serde_json::to_string_pretty(v)?).context("writing credentials file")?;
    Ok(())
}

pub fn save_api_key(key: &str) -> Result<()> {
    let mut v = load_credentials_json();
    v["openai_api_key"] = serde_json::Value::String(key.to_string());
    save_credentials_json(&v)
}

pub fn load_api_key() -> Option<String> {
    load_credentials_json()
        .get("openai_api_key")
        .and_then(|x| x.as_str().map(|s| s.to_string()))
}

pub fn save_anthropic_api_key(key: &str) -> Result<()> {
    let mut v = load_credentials_json();
    v["anthropic_api_key"] = serde_json::Value::String(key.to_string());
    save_credentials_json(&v)
}

pub fn load_anthropic_api_key() -> Option<String> {
    load_credentials_json()
        .get("anthropic_api_key")
        .and_then(|x| x.as_str().map(|s| s.to_string()))
}

fn load_cache_or_default() -> CacheData {
    load_cache().unwrap_or(CacheData {
        openai_total: None,
        fetched_at: None,
        baseline: None,
        baseline_used: None,
        baseline_remaining: None,
        anthropic_total: None,
        anthropic_fetched_at: None,
        anthropic_baseline: None,
        anthropic_baseline_used: None,
        anthropic_baseline_remaining: None,
    })
}

fn write_cache(data: &CacheData) -> Result<()> {
    let p = cache_path().context("cannot find config dir")?;
    fs::create_dir_all(p.parent().unwrap()).ok();
    fs::write(&p, serde_json::to_string_pretty(data)?).context("writing cache file")?;
    Ok(())
}

pub fn save_cache(total: &Decimal) -> Result<()> {
    let mut data = load_cache_or_default();
    data.openai_total = Some(total.to_string());
    data.fetched_at = Some(chrono::Utc::now().to_rfc3339());
    write_cache(&data)
}

pub fn save_baseline_cache(used: &Decimal, remaining: &Decimal) -> Result<()> {
    let mut data = load_cache_or_default();
    data.baseline_used = Some(used.to_string());
    data.baseline_remaining = Some(remaining.to_string());
    data.fetched_at = Some(chrono::Utc::now().to_rfc3339());
    write_cache(&data)
}

pub fn save_anthropic_cache(total: &Decimal) -> Result<()> {
    let mut data = load_cache_or_default();
    data.anthropic_total = Some(total.to_string());
    data.anthropic_fetched_at = Some(chrono::Utc::now().to_rfc3339());
    write_cache(&data)
}

pub fn save_anthropic_baseline_cache(used: &Decimal, remaining: &Decimal) -> Result<()> {
    let mut data = load_cache_or_default();
    data.anthropic_baseline_used = Some(used.to_string());
    data.anthropic_baseline_remaining = Some(remaining.to_string());
    data.anthropic_fetched_at = Some(chrono::Utc::now().to_rfc3339());
    write_cache(&data)
}

pub fn load_cache() -> Option<CacheData> {
    let p = cache_path()?;
    if !p.exists() {
        return None;
    }
    match fs::read_to_string(&p) {
        Ok(txt) => match serde_json::from_str::<CacheData>(&txt) {
            Ok(c) => Some(c),
            Err(e) => { eprintln!("[storage] load_cache deserialization error: {}", e); None },
        },
        Err(e) => { eprintln!("[storage] load_cache read error: {}", e); None },
    }
}

fn cache_age_seconds(fetched_at: &Option<String>) -> Option<i64> {
    let ts = fetched_at.as_deref()?;
    let dt = chrono::DateTime::parse_from_rfc3339(ts).ok()?;
    let age = chrono::Utc::now().signed_duration_since(dt.with_timezone(&chrono::Utc));
    Some(age.num_seconds())
}

pub fn is_cache_outdated() -> bool {
    load_cache()
        .and_then(|c| cache_age_seconds(&c.fetched_at))
        .map(|secs| secs >= 180)
        .unwrap_or(true)
}

pub fn is_anthropic_cache_outdated() -> bool {
    load_cache()
        .and_then(|c| cache_age_seconds(&c.anthropic_fetched_at))
        .map(|secs| secs >= 180)
        .unwrap_or(true)
}

pub fn save_baseline(amount: &str, start_iso: &str) -> Result<()> {
    let mut data = load_cache_or_default();
    data.baseline = Some(Baseline { amount: amount.to_string(), start_iso: start_iso.to_string() });
    data.baseline_used = None;
    data.baseline_remaining = None;
    write_cache(&data)
}

pub fn clear_baseline() -> Result<()> {
    let mut data = load_cache_or_default();
    data.baseline = None;
    data.baseline_used = None;
    data.baseline_remaining = None;
    write_cache(&data)
}

pub fn save_anthropic_baseline(amount: &str, start_iso: &str) -> Result<()> {
    let mut data = load_cache_or_default();
    data.anthropic_baseline = Some(Baseline { amount: amount.to_string(), start_iso: start_iso.to_string() });
    data.anthropic_baseline_used = None;
    data.anthropic_baseline_remaining = None;
    write_cache(&data)
}

pub fn clear_anthropic_baseline() -> Result<()> {
    let mut data = load_cache_or_default();
    data.anthropic_baseline = None;
    data.anthropic_baseline_used = None;
    data.anthropic_baseline_remaining = None;
    write_cache(&data)
}
