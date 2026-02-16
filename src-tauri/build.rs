use std::fs;
use std::path::Path;

fn main() {
  // Skip config modification in dev mode to avoid infinite rebuild loops
  // Check if we're in debug build (dev mode) instead of environment variables
  let is_debug = std::env::var("PROFILE").unwrap_or_default() == "debug";
  
  if is_debug {
    eprintln!("[build.rs] debug build detected, skipping config modification");
    tauri_build::build();
    return;
  }

  // Detect target OS (CARGO_CFG_TARGET_OS is preferable, fallback to TARGET triple)
  let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| std::env::var("TARGET").unwrap_or_default());
  let is_linux = target_os.contains("linux");

  let conf_path = Path::new("tauri.conf.json");

  // Read original config so we can restore it later
  let original = fs::read_to_string(conf_path).expect("failed to read tauri.conf.json");

  // Parse and modify transparent flag for the main window based on platform
  match serde_json::from_str::<serde_json::Value>(&original) {
    Ok(mut json) => {
      if let Some(app) = json.get_mut("app") {
        if let Some(windows) = app.get_mut("windows").and_then(|w| w.as_array_mut()) {
          if let Some(first) = windows.get_mut(0) {
            // On Linux, set transparent = false; on other platforms keep true
            let new_transparent = serde_json::Value::Bool(!is_linux);
            first["transparent"] = new_transparent.clone();
          }
        }
      }
      // Serialize modified config and log it for debug purposes
      let modified = serde_json::to_string_pretty(&json).expect("failed to serialize modified tauri.conf.json");
      eprintln!("[build.rs] target_os='{}' is_linux={} — writing modified tauri.conf.json:\n{}", target_os, is_linux, modified);
      fs::write(conf_path, &modified).expect("failed to write modified tauri.conf.json");
    }
    Err(e) => {
      eprintln!("[build.rs] failed to parse tauri.conf.json: {} — proceeding without modification", e);
    }
  }

  // Run the normal tauri build step
  eprintln!("[build.rs] invoking tauri_build::build()...");
  tauri_build::build();
  eprintln!("[build.rs] tauri_build::build() finished");

  // Restore original config to avoid leaving repo modified
  fs::write(conf_path, original).expect("failed to restore original tauri.conf.json");
  eprintln!("[build.rs] restored original tauri.conf.json");
}
