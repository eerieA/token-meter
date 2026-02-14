#[path = "../../src/providers/mod.rs"]
mod providers;
#[path = "../../src/aggregator.rs"]
mod aggregator;
#[path = "../../src/storage.rs"]
mod storage;
#[path = "../../src/domain.rs"]
mod domain;

use storage::{is_cache_outdated, load_api_key, load_cache, save_api_key, save_cache, save_baseline, clear_baseline};
use tauri::Manager;

#[tauri::command]
async fn move_window(window: tauri::Window, x: f64, y: f64) -> Result<(), String> {
  window
    .set_position(tauri::Position::Physical(tauri::PhysicalPosition {
      x: x as i32,
      y: y as i32,
    }))
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_api_key() -> Result<Option<String>, String> {
  Ok(load_api_key())
}

#[tauri::command]
async fn get_cached_data() -> Result<Option<serde_json::Value>, String> {
  match load_cache() {
    Some(cache) => {
      if !is_cache_outdated() {
        if let Some(total) = cache.openai_total {
          // Include baseline info if present so the UI can show baseline metadata without hitting the network
          if let Some(b) = cache.baseline {
            Ok(Some(serde_json::json!({
              "success": true,
              "data": total,
              "baseline": {
                "amount": b.amount,
                "start_iso": b.start_iso
              },
              "status": "From cache"
            })))
          } else {
            Ok(Some(serde_json::json!({
              "success": true,
              "data": total,
              "status": "From cache"
            })))
          }
        } else {
          Ok(None)
        }
      } else {
        Ok(None)
      }
    }
    None => Ok(None),
  }
}

#[tauri::command]
async fn fetch_month_to_date(api_key: String) -> Result<serde_json::Value, String> {
  let agg = aggregator::UsageAggregator::new(&api_key);

  // If there's a baseline configured in cache, compute usage since baseline start and return remaining amount
  if let Some(cache) = load_cache() {
    if let Some(baseline) = cache.baseline {
      // parse baseline start ISO
      match chrono::DateTime::parse_from_rfc3339(&baseline.start_iso) {
        Ok(dt) => {
          let start_ts = dt.with_timezone(&chrono::Utc).timestamp();
          let end_ts = chrono::Utc::now().timestamp();
          match agg.fetch_since(start_ts, Some(end_ts)).await {
            Ok(used) => {
              // parse baseline amount
              match rust_decimal::Decimal::from_str_exact(&baseline.amount) {
                Ok(bamount) => {
                  let remaining = bamount - used;
                  Ok(serde_json::json!({
                    "success": true,
                    "data": remaining.to_string(),
                    "status": "From baseline"
                  }))
                }
                Err(e) => Ok(serde_json::json!({
                  "success": false,
                  "error": format!("invalid baseline amount: {}", e)
                })),
              }
            }
            Err(e) => Ok(serde_json::json!({
              "success": false,
              "error": e.to_string()
            })),
          }
        }
        Err(e) => Ok(serde_json::json!({
          "success": false,
          "error": format!("invalid baseline start_iso: {}", e)
        })),
      }
    } else {
      // No baseline: default month-to-date behavior
      match agg.fetch_month_to_date().await {
        Ok(total) => {
          // Save to cache after successful fetch
          if let Err(e) = save_cache(&total) {
            eprintln!("Failed to save cache: {}", e);
          }
          Ok(serde_json::json!({
            "success": true,
            "data": total.to_string()
          }))
        },
        Err(e) => Ok(serde_json::json!({
          "success": false,
          "error": e.to_string()
        })),
      }
    }
  } else {
    // No cache present; behave like month-to-date
    match agg.fetch_month_to_date().await {
      Ok(total) => Ok(serde_json::json!({
        "success": true,
        "data": total.to_string()
    })),
      Err(e) => Ok(serde_json::json!({
        "success": false,
        "error": e.to_string()
      })),
    }
  }
}

#[tauri::command]
async fn validate_api_key(api_key: String) -> Result<serde_json::Value, String> {
  let agg = aggregator::UsageAggregator::new(&api_key);
  match agg.fetch_month_to_date().await {
    Ok(_) => Ok(serde_json::json!({
      "success": true,
      "message": "API key is valid"
    })),
    Err(e) => Ok(serde_json::json!({
      "success": false,
      "error": e.to_string()
    })),
  }
}

#[tauri::command]
fn save_api_key_command(api_key: String) -> Result<(), String> {
  save_api_key(&api_key).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_baseline_command(amount: String, start_iso: String) -> Result<(), String> {
  save_baseline(&amount, &start_iso).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_baseline_command() -> Result<(), String> {
  clear_baseline().map_err(|e| e.to_string())
}

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      move_window,
      get_api_key,
      get_cached_data,
      fetch_month_to_date,
      validate_api_key,
      save_api_key_command,
      save_baseline_command,
      clear_baseline_command,
    ])
    .setup(|app| {
      // Open dev tools automatically in debug mode
      #[cfg(debug_assertions)]
      {
        let window = app.get_webview_window("main").unwrap();
        window.open_devtools();
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
