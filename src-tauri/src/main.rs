#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[path = "../../src/providers/mod.rs"]
mod providers;
#[path = "../../src/aggregator.rs"]
mod aggregator;
#[path = "../../src/storage.rs"]
mod storage;
#[path = "../../src/domain.rs"]
mod domain;

use storage::{
    is_cache_outdated, is_anthropic_cache_outdated,
    load_api_key, load_anthropic_api_key,
    load_cache,
    save_api_key, save_anthropic_api_key,
    save_cache, save_baseline_cache,
    save_anthropic_cache, save_anthropic_baseline_cache,
    save_baseline, clear_baseline,
    save_anthropic_baseline, clear_anthropic_baseline,
};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder, PhysicalPosition};
use std::sync::Mutex;

struct BaselineProviderState(Mutex<String>);

// ── OpenAI commands ──────────────────────────────────────────────────────────

#[tauri::command]
async fn get_api_key() -> Result<Option<String>, String> {
    Ok(load_api_key())
}

#[tauri::command]
async fn get_cached_data() -> Result<Option<serde_json::Value>, String> {
    let Some(cache) = load_cache() else { return Ok(None); };

    // For each provider: always return baseline config if present (so the UI and fetch commands
    // know the baseline exists even before a first successful fetch). Only gate the pre-computed
    // cost data (total / used / remaining) on cache freshness.
    let openai_fresh = !is_cache_outdated();
    let openai_val: Option<serde_json::Value> = if let Some(b) = &cache.baseline {
        let mut baseline_obj = serde_json::json!({ "amount": b.amount, "start_iso": b.start_iso });
        if openai_fresh {
            if let Some(u) = &cache.baseline_used { baseline_obj["used"] = u.clone().into(); }
            if let Some(r) = &cache.baseline_remaining { baseline_obj["remaining"] = r.clone().into(); }
        }
        let data_value = if openai_fresh {
            cache.baseline_remaining.as_deref()
                .or(cache.openai_total.as_deref())
                .map(|s| serde_json::Value::String(s.to_string()))
                .unwrap_or(serde_json::Value::Null)
        } else {
            serde_json::Value::Null
        };
        Some(serde_json::json!({ "success": true, "data": data_value, "baseline": baseline_obj, "status": "From cache" }))
    } else if openai_fresh {
        cache.openai_total.as_ref().map(|total| {
            serde_json::json!({ "success": true, "data": total, "status": "From cache" })
        })
    } else {
        None
    };

    let anthropic_fresh = !is_anthropic_cache_outdated();
    let anthropic_val: Option<serde_json::Value> = if let Some(b) = &cache.anthropic_baseline {
        let mut baseline_obj = serde_json::json!({ "amount": b.amount, "start_iso": b.start_iso });
        if anthropic_fresh {
            if let Some(u) = &cache.anthropic_baseline_used { baseline_obj["used"] = u.clone().into(); }
            if let Some(r) = &cache.anthropic_baseline_remaining { baseline_obj["remaining"] = r.clone().into(); }
        }
        let data_value = if anthropic_fresh {
            cache.anthropic_baseline_remaining.as_deref()
                .or(cache.anthropic_total.as_deref())
                .map(|s| serde_json::Value::String(s.to_string()))
                .unwrap_or(serde_json::Value::Null)
        } else {
            serde_json::Value::Null
        };
        Some(serde_json::json!({ "success": true, "data": data_value, "baseline": baseline_obj, "status": "From cache" }))
    } else if anthropic_fresh {
        cache.anthropic_total.as_ref().map(|total| {
            serde_json::json!({ "success": true, "data": total, "status": "From cache" })
        })
    } else {
        None
    };

    if openai_val.is_none() && anthropic_val.is_none() {
        return Ok(None);
    }

    Ok(Some(serde_json::json!({
        "openai": openai_val,
        "anthropic": anthropic_val,
    })))
}

#[tauri::command]
async fn fetch_month_to_date(api_key: String) -> Result<serde_json::Value, String> {
    let agg = aggregator::UsageAggregator::new_openai(&api_key);

    if let Some(cache) = load_cache() {
        if let Some(baseline) = cache.baseline {
            match chrono::DateTime::parse_from_rfc3339(&baseline.start_iso) {
                Ok(dt) => {
                    let start_ts = dt.with_timezone(&chrono::Utc).timestamp();
                    eprintln!("[main] fetch_month_to_date (openai): baseline branch start_ts={}", start_ts);
                    match agg.fetch_since(start_ts, None).await {
                        Ok(used) => {
                            match rust_decimal::Decimal::from_str_exact(&baseline.amount) {
                                Ok(bamount) => {
                                    let remaining = bamount - used;
                                    if let Err(e) = save_baseline_cache(&used, &remaining) {
                                        eprintln!("Failed to save openai baseline cache: {}", e);
                                    }
                                    Ok(serde_json::json!({ "success": true, "data": remaining.to_string(), "status": "With baseline" }))
                                }
                                Err(e) => Ok(serde_json::json!({ "success": false, "error": format!("invalid baseline amount: {}", e) })),
                            }
                        }
                        Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() })),
                    }
                }
                Err(e) => Ok(serde_json::json!({ "success": false, "error": format!("invalid baseline start_iso: {}", e) })),
            }
        } else {
            eprintln!("[main] fetch_month_to_date (openai): no baseline");
            match agg.fetch_month_to_date().await {
                Ok(total) => {
                    if let Err(e) = save_cache(&total) { eprintln!("Failed to save openai cache: {}", e); }
                    Ok(serde_json::json!({ "success": true, "data": total.to_string(), "status": "Fetched" }))
                }
                Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() })),
            }
        }
    } else {
        match agg.fetch_month_to_date().await {
            Ok(total) => Ok(serde_json::json!({ "success": true, "data": total.to_string(), "status": "Fetched" })),
            Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() })),
        }
    }
}

#[tauri::command]
async fn validate_api_key(api_key: String) -> Result<serde_json::Value, String> {
    let agg = aggregator::UsageAggregator::new_openai(&api_key);
    match agg.fetch_month_to_date().await {
        Ok(_) => Ok(serde_json::json!({ "success": true, "message": "API key is valid" })),
        Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() })),
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

// ── Anthropic commands ────────────────────────────────────────────────────────

#[tauri::command]
async fn get_anthropic_api_key() -> Result<Option<String>, String> {
    Ok(load_anthropic_api_key())
}

#[tauri::command]
fn save_anthropic_api_key_command(api_key: String) -> Result<(), String> {
    save_anthropic_api_key(&api_key).map_err(|e| e.to_string())
}

#[tauri::command]
async fn validate_anthropic_api_key(api_key: String) -> Result<serde_json::Value, String> {
    let agg = aggregator::UsageAggregator::new_anthropic(&api_key);
    match agg.fetch_month_to_date_anthropic().await {
        Ok(_) => Ok(serde_json::json!({ "success": true, "message": "API key is valid" })),
        Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() })),
    }
}

#[tauri::command]
async fn fetch_anthropic_month_to_date(api_key: String) -> Result<serde_json::Value, String> {
    let agg = aggregator::UsageAggregator::new_anthropic(&api_key);

    if let Some(cache) = load_cache() {
        if let Some(baseline) = cache.anthropic_baseline {
            match chrono::DateTime::parse_from_rfc3339(&baseline.start_iso) {
                Ok(dt) => {
                    let start_ts = dt.with_timezone(&chrono::Utc).timestamp();
                    eprintln!("[main] fetch_month_to_date (anthropic): baseline branch start_ts={}", start_ts);
                    match agg.fetch_since_anthropic(start_ts, None).await {
                        Ok(used) => {
                            match rust_decimal::Decimal::from_str_exact(&baseline.amount) {
                                Ok(bamount) => {
                                    let remaining = bamount - used;
                                    if let Err(e) = save_anthropic_baseline_cache(&used, &remaining) {
                                        eprintln!("Failed to save anthropic baseline cache: {}", e);
                                    }
                                    Ok(serde_json::json!({ "success": true, "data": remaining.to_string(), "status": "With baseline" }))
                                }
                                Err(e) => Ok(serde_json::json!({ "success": false, "error": format!("invalid baseline amount: {}", e) })),
                            }
                        }
                        Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() })),
                    }
                }
                Err(e) => Ok(serde_json::json!({ "success": false, "error": format!("invalid baseline start_iso: {}", e) })),
            }
        } else {
            eprintln!("[main] fetch_month_to_date (anthropic): no baseline");
            match agg.fetch_month_to_date_anthropic().await {
                Ok(total) => {
                    if let Err(e) = save_anthropic_cache(&total) { eprintln!("Failed to save anthropic cache: {}", e); }
                    Ok(serde_json::json!({ "success": true, "data": total.to_string(), "status": "Fetched" }))
                }
                Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() })),
            }
        }
    } else {
        match agg.fetch_month_to_date_anthropic().await {
            Ok(total) => Ok(serde_json::json!({ "success": true, "data": total.to_string(), "status": "Fetched" })),
            Err(e) => Ok(serde_json::json!({ "success": false, "error": e.to_string() })),
        }
    }
}

#[tauri::command]
fn save_anthropic_baseline_command(amount: String, start_iso: String) -> Result<(), String> {
    save_anthropic_baseline(&amount, &start_iso).map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_anthropic_baseline_command() -> Result<(), String> {
    clear_anthropic_baseline().map_err(|e| e.to_string())
}

// ── Baseline provider state (shared between main and baseline-modal windows) ──

#[tauri::command]
fn set_baseline_provider(state: tauri::State<BaselineProviderState>, provider: String) {
    *state.0.lock().unwrap() = provider;
}

#[tauri::command]
fn get_baseline_provider(state: tauri::State<BaselineProviderState>) -> String {
    state.0.lock().unwrap().clone()
}

// ── Window / app commands ─────────────────────────────────────────────────────

#[tauri::command]
async fn close_window(window: tauri::Window) -> Result<(), String> {
    window.close().map_err(|e| e.to_string())
}

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    eprintln!("[tauri] quit_app invoked");
    app.exit(0);
}

#[tauri::command]
fn show_context_menu(app: tauri::AppHandle, x: i32, y: i32) -> Result<(), String> {
    eprintln!("[tauri] show_context_menu invoked x={} y={}", x, y);
    if let Some(win) = app.get_webview_window("context-menu") {
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
        .manage(BaselineProviderState(Mutex::new("openai".to_string())))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_api_key,
            get_cached_data,
            fetch_month_to_date,
            validate_api_key,
            save_api_key_command,
            save_baseline_command,
            clear_baseline_command,
            get_anthropic_api_key,
            save_anthropic_api_key_command,
            validate_anthropic_api_key,
            fetch_anthropic_month_to_date,
            save_anthropic_baseline_command,
            clear_anthropic_baseline_command,
            set_baseline_provider,
            get_baseline_provider,
            close_window,
            quit_app,
            show_context_menu,
            hide_context_menu,
            show_baseline_modal,
            hide_baseline_modal,
        ])
        .setup(|app| {
            let overlay = WebviewWindowBuilder::new(
                app,
                "context-menu",
                WebviewUrl::App("overlay.html".into()),
            )
            .title("")
            .decorations(false)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .visible(false)
            .transparent(true)
            .focused(true)
            .focusable(true)
            .accept_first_mouse(true)
            .inner_size(96.0, 64.0)
            .build()?;

            let baseline_overlay = WebviewWindowBuilder::new(
                app,
                "baseline-modal",
                WebviewUrl::App("baseline.html".into()),
            )
            .title("")
            .decorations(false)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .visible(false)
            .transparent(true)
            .focused(true)
            .focusable(true)
            .accept_first_mouse(true)
            .inner_size(230.0, 120.0)
            .build()?;

            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
                overlay.open_devtools();
                baseline_overlay.open_devtools();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
