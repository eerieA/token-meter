#[path = "../../src/providers/mod.rs"]
mod providers;
#[path = "../../src/aggregator.rs"]
mod aggregator;
#[path = "../../src/storage.rs"]
mod storage;
#[path = "../../src/domain.rs"]
mod domain;

use storage::{is_cache_outdated, load_api_key, load_cache, save_api_key, save_cache, save_baseline, clear_baseline, save_baseline_cache};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, PhysicalPosition};

#[tauri::command]
async fn get_api_key() -> Result<Option<String>, String> {
  Ok(load_api_key())
}

#[tauri::command]
async fn get_cached_data() -> Result<Option<serde_json::Value>, String> {
  match load_cache() {
    Some(cache) => {
      if !is_cache_outdated() {
        // If baseline metadata exists, prefer returning baseline-related cached values
        if let Some(b) = &cache.baseline {
          let mut baseline_obj = serde_json::json!({
            "amount": b.amount,
            "start_iso": b.start_iso
          });
          if let Some(used) = &cache.baseline_used {
            baseline_obj["used"] = serde_json::Value::String(used.clone());
          }
          if let Some(rem) = &cache.baseline_remaining {
            baseline_obj["remaining"] = serde_json::Value::String(rem.clone());
          }
          // Choose data to return: prefer baseline_remaining if present, otherwise openai_total if present
          let data_value = if let Some(ref rem) = cache.baseline_remaining {
            serde_json::Value::String(rem.clone())
          } else if let Some(ref total) = cache.openai_total {
            serde_json::Value::String(total.clone())
          } else {
            serde_json::Value::Null
          };

          Ok(Some(serde_json::json!({
            "success": true,
            "data": data_value,
            "baseline": baseline_obj,
            "status": "From cache"
          })))
        } else if let Some(total) = cache.openai_total {
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

  // If there's a baseline configured in cache, compute usage since baseline start and return remaining amount
  if let Some(cache) = load_cache() {
    if let Some(baseline) = cache.baseline {
      // parse baseline start ISO
      match chrono::DateTime::parse_from_rfc3339(&baseline.start_iso) {
        Ok(dt) => {
          let start_ts = dt.with_timezone(&chrono::Utc).timestamp();
          let end_ts = chrono::Utc::now().timestamp();
          eprintln!("[main] fetch_month_to_date: using baseline branch start_ts={} end_ts={} (passing end_time=None to provider to include latest bucket)", start_ts, end_ts);
          // Pass None as end_time to provider so it can decide the appropriate upper bound
          match agg.fetch_since(start_ts, None).await {
            Ok(used) => {
              // parse baseline amount
              match rust_decimal::Decimal::from_str_exact(&baseline.amount) {
                Ok(bamount) => {
                  let remaining = bamount - used;
                  // Save baseline-specific aggregates (used & remaining) without overwriting openai_total
                  if let Err(e) = save_baseline_cache(&used, &remaining) {
                    eprintln!("Failed to save baseline cache: {}", e);
                  } else {
                    eprintln!("[main] baseline cache saved (used={}, remaining={})", used, remaining);
                  }
                  Ok(serde_json::json!({
                    "success": true,
                    "data": remaining.to_string(),
                    "status": "With baseline"
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
      eprintln!("[main] fetch_month_to_date: no baseline, calling month-to-date provider");
      match agg.fetch_month_to_date().await {
        Ok(total) => {
          // Save to cache after successful fetch
          if let Err(e) = save_cache(&total) {
            eprintln!("Failed to save cache: {}", e);
          } else {
            eprintln!("[main] cache saved successfully (openai_total={})", total);
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

#[tauri::command]
async fn close_window(window: tauri::Window) -> Result<(), String> {
  window.close().map_err(|e| e.to_string())
}

// Close the entire application (used by the overlay 'Close' menu item)
#[tauri::command]
fn quit_app() {
  eprintln!("[tauri] quit_app invoked");
  // Best-effort: exit the process entirely
  std::process::exit(0);
}

// Commands for the native context-menu overlay window
#[tauri::command]
fn show_context_menu(app: tauri::AppHandle, x: i32, y: i32) -> Result<(), String> {
  eprintln!("[tauri] show_context_menu invoked x={} y={}", x, y);
  if let Some(win) = app.get_webview_window("context-menu") {
    // Position and show the overlay window. Adjust position to avoid negative coords.
    let px = if x < 0 { 0 } else { x };
    let py = if y < 0 { 0 } else { y };
    let _ = win.set_position(PhysicalPosition::new(px, py));
    let _ = win.show();
    let _ = win.set_focus();
    return Ok(());
  }
  Err("context-menu window not found".into())
}

#[tauri::command]
fn hide_context_menu(app: tauri::AppHandle) -> Result<(), String> {
  eprintln!("[tauri] hide_context_menu invoked");
  if let Some(win) = app.get_webview_window("context-menu") {
    let _ = win.hide();
    return Ok(());
  }
  Err("context-menu window not found".into())
}

#[tauri::command]
fn show_baseline_modal(app: tauri::AppHandle, x: i32, y: i32) -> Result<(), String> {
  eprintln!("[tauri] show_baseline_modal invoked x={} y={}", x, y);
  if let Some(win) = app.get_webview_window("baseline-modal") {
    let px = if x < 0 { 0 } else { x };
    let py = if y < 0 { 0 } else { y };
    let _ = win.set_position(PhysicalPosition::new(px, py));
    let _ = win.show();
    let _ = win.set_focus();
    return Ok(());
  }
  Err("baseline-modal window not found".into())
}

#[tauri::command]
fn hide_baseline_modal(app: tauri::AppHandle) -> Result<(), String> {
  eprintln!("[tauri] hide_baseline_modal invoked");
  if let Some(win) = app.get_webview_window("baseline-modal") {
    let _ = win.hide();
    return Ok(());
  }
  Err("baseline-modal window not found".into())
}

fn main() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      get_api_key,
      get_cached_data,
      fetch_month_to_date,
      validate_api_key,
      save_api_key_command,
      save_baseline_command,
      clear_baseline_command,
      close_window,
      quit_app,
      show_context_menu,
      hide_context_menu,
      show_baseline_modal,
      hide_baseline_modal,
    ])
    .setup(|app| {
      // Create a hidden overlay window for the context menu using WebviewWindowBuilder
      let overlay = WebviewWindowBuilder::new(
        app,
        "context-menu",
        WebviewUrl::App("overlay.html".into()),
      )
      .title("")
      .decorations(false)
      .always_on_top(true)
      .skip_taskbar(true)
      .resizable(false)
      .visible(false)
      .transparent(true)
      .focused(true)
      .focusable(true)
      .accept_first_mouse(true)
      .inner_size(180.0, 120.0)
      .build()?;

      // Create a hidden overlay window for the baseline modal (covers a portion of the app)
      let baseline_overlay = WebviewWindowBuilder::new(
        app,
        "baseline-modal",
        WebviewUrl::App("baseline.html".into()),
      )
      .title("")
      .decorations(false)
      .always_on_top(true)
      .skip_taskbar(true)
      .resizable(false)
      .visible(false)
      .transparent(true)
      .focused(true)
      .focusable(true)
      .accept_first_mouse(true)
      // Give it a reasonable default size; the frontend will position it to match the main window
      .inner_size(360.0, 220.0)
      .build()?;

      // Open dev tools automatically in debug mode for main and overlay windows
      #[cfg(debug_assertions)]
      {
        if let Some(window) = app.get_webview_window("main") {
          window.open_devtools();
        }
        // Open devtools for overlays to help debugging
        overlay.open_devtools();
        baseline_overlay.open_devtools();
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
