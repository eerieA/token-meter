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
}

#[derive(Debug, Serialize, Deserialize)]
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

pub fn save_api_key(key: &str) -> Result<()> {
    let mut p = base_dir().context("cannot find config dir")?;
    fs::create_dir_all(&p).ok();
    p.push("credentials.json");
    let j = serde_json::json!({"openai_api_key": key});
    fs::write(&p, serde_json::to_string_pretty(&j)?).context("writing credentials file")?;
    Ok(())
}

pub fn load_api_key() -> Option<String> {
    let mut p = base_dir()?;
    p.push("credentials.json");
    if !p.exists() {
        return None;
    }
    match fs::read_to_string(&p) {
        Ok(txt) => match serde_json::from_str::<serde_json::Value>(&txt) {
            Ok(v) => v.get("openai_api_key").and_then(|x| x.as_str().map(|s| s.to_string())),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

pub fn save_cache(total: &Decimal) -> Result<()> {
    let mut p = base_dir().context("cannot find config dir")?;
    fs::create_dir_all(&p).ok();
    p.push("api_usage.json");
    
    // Load existing cache data or create new
    let mut data = load_cache().unwrap_or(CacheData { 
        openai_total: None, 
        fetched_at: None, 
        baseline: None 
    });
    
    // Update the cache data
    data.openai_total = Some(total.to_string());
    data.fetched_at = Some(chrono::Utc::now().to_rfc3339());
    
    fs::write(&p, serde_json::to_string_pretty(&data)?).context("writing cache file")?;
    Ok(())
}

pub fn load_cache() -> Option<CacheData> {
    let mut p = base_dir()?;
    p.push("api_usage.json");
    if !p.exists() {
        return None;
    }
    match fs::read_to_string(&p) {
        Ok(txt) => match serde_json::from_str::<CacheData>(&txt) {
            Ok(c) => Some(c),
            Err(_) => None,
        },
        Err(_) => None,
    }
}

pub fn is_cache_outdated() -> bool {
    if let Some(cache) = load_cache() {
        if let Some(fetched_at) = cache.fetched_at {
            match chrono::DateTime::parse_from_rfc3339(&fetched_at) {
                Ok(dt) => {
                    let cache_age = chrono::Utc::now() - chrono::DateTime::from_naive_utc_and_offset(dt.naive_utc(), chrono::Utc);
                    cache_age.num_minutes() >= 3 // Cache is outdated after 3 minutes
                }
                Err(_) => true, // Invalid timestamp, treat as outdated
            }
        } else {
            true // No timestamp, treat as outdated
        }
    } else {
        true // No cache, treat as outdated
    }
}

pub fn save_baseline(amount: &str, start_iso: &str) -> Result<()> {
    let mut p = base_dir().context("cannot find config dir")?;
    fs::create_dir_all(&p).ok();
    p.push("api_usage.json");
    // load existing
    let mut data = load_cache().unwrap_or(CacheData { openai_total: None, fetched_at: None, baseline: None });
    data.baseline = Some(Baseline { amount: amount.to_string(), start_iso: start_iso.to_string() });
    fs::write(&p, serde_json::to_string_pretty(&data)?).context("writing cache file")?;
    Ok(())
}
