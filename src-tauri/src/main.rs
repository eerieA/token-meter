#[path = "../../src/providers/mod.rs"]
mod providers;
#[path = "../../src/aggregator.rs"]
mod aggregator;
#[path = "../../src/storage.rs"]
mod storage;
#[path = "../../src/domain.rs"]
mod domain;

use storage::{is_cache_outdated, load_api_key, load_cache, save_api_key, save_cache};
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
          Ok(Some(serde_json::json!({
            "success": true,
            "data": total,
            "status": "From cache"
          })))
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

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      move_window,
      get_api_key,
      get_cached_data,
      fetch_month_to_date,
      validate_api_key,
      save_api_key_command,
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
